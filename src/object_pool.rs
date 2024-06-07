pub mod object_pool {

/// Poolable trait that must be implemented by objects that will be stored in the pool.
/// The trait provides a way to create a new object and reset it to its initial state.
pub trait Poolable {
    /// Create a new instance of the object.
    fn new() -> Self;
    
    /// Reset the object to its initial state.
    fn reset(&mut self);
}

/// Raw variant of ObjectPool that allows manual get and release of items.
pub struct ObjectPool<T: Poolable> {
    pub items: Vec<Box<T>>,
    pub available: Vec<*mut T>
}

impl<T: Poolable> ObjectPool<T> {
    /// Create a new ObjectPool.
    pub fn new() -> ObjectPool<T> {
        ObjectPool {
            items: Vec::new(),
            available: Vec::new()
        }
    }

    /// Reserve a number of items in the pool.
    pub fn reserve(&mut self, count: usize) {
        for _ in 0..count {
            let mut item = Box::new(T::new());
            let ptr = &mut *item as *mut T;
            self.items.push(item);
            self.available.push(ptr);
        }
    }

    /// Get an item from the pool.
    pub fn get(&mut self) -> *mut T {
        if self.available.is_empty() {
            let mut item = Box::new(T::new());
            let ptr = &mut *item as *mut T;
            self.items.push(item);
            ptr
        } else {
            let ptr = self.available.pop().unwrap();
            ptr
        }
    }

    /// Release an item back to the pool.
    pub fn release(&mut self, item: *mut T) {
        unsafe {
            (*item).reset();
        }
        self.available.push(item);
    }

    /// Clear the pool completely.
    pub fn clear(&mut self) {
        self.items.clear();
        self.available.clear();
    }

    /// Release all items back to the pool.
    pub fn release_all(&mut self) {
        for item in self.items.iter_mut() {
            item.reset();
            self.available.push(&mut **item as *mut T);
        }
    }

    /// Get the number of items in the pool.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get the number of available items in the pool.
    pub fn available(&self) -> usize {
        self.available.len()
    }
}

/// PoolBox is a wrapper around a pool item that automatically releases the item back to the pool when dropped.
pub struct PoolBox<T: Poolable> {
    pub item: *mut T,
    pub pool: *mut ObjectPool<T>
}

impl<T: Poolable> PoolBox<T> {
    /// Create a new PoolBox that wraps an item from the pool.
    pub fn new(pool: *mut ObjectPool<T>) -> PoolBox<T> {
        let item = unsafe {
            (*pool).get()
        };
        PoolBox {
            item: item,
            pool: pool
        }
    }

    /// Get a reference to the item.
    pub fn ref_item(&self) -> &T {
        unsafe {
            &*self.item
        }
    }

    /// Get a mutable reference to the item.
    pub fn ref_mut_item(&mut self) -> &mut T {
        unsafe {
            &mut *self.item
        }
    }

    /// Extract the item from the PoolBox without releasing it back to the pool.
    /// The raw pool will be responsible for releasing the item.
    pub fn extract(&mut self) -> *mut T {
        let item = self.item;
        self.item = std::ptr::null_mut();
        item
    }
}

impl<T: Poolable> Drop for PoolBox<T> {
    fn drop(&mut self) {
        if self.item.is_null() {
            return;
        }
        unsafe {
            (*self.pool).release(self.item);
        }
    }
}

pub struct AutoReturnObjectPool<T: Poolable> {
    pub pool: ObjectPool<T>
}

impl<T: Poolable> AutoReturnObjectPool<T> {
    /// Create a new AutoReturnObjectPool.
    pub fn new() -> AutoReturnObjectPool<T> {
        AutoReturnObjectPool {
            pool: ObjectPool::new()
        }
    }

    /// Get a PoolBox from the pool.
    pub fn get(&mut self) -> PoolBox<T> {
        PoolBox::new(&mut self.pool)
    }
}

}

#[cfg(test)]
mod tests {
    use super::object_pool::*;

    struct TestObject {
        pub value: i32
    }

    impl Poolable for TestObject {
        fn new() -> TestObject {
            TestObject {
                value: 0
            }
        }

        fn reset(&mut self) {
            self.value = 0;
        }
    }

    #[test]
    fn test_object_pool() {
        let mut pool = AutoReturnObjectPool::<TestObject>::new();
        {
            let mut obj = pool.get();
            obj.ref_mut_item().value = 10;
            assert_eq!(obj.ref_item().value, 10);
            assert_eq!(pool.pool.len(), 1);
        }
        assert_eq!(pool.pool.available(), 1);
        assert!(pool.pool.len() > 0);
        {
            // Create 10 objects
            let mut objs = Vec::new();
            for _ in 0..10 {
                objs.push(pool.get());
            }
            assert_eq!(pool.pool.available(), 0);
            assert_eq!(pool.pool.len(), 10);
        }
        assert_eq!(pool.pool.available(), 10);
        assert_eq!(pool.pool.len(), 10);
        pool.pool.clear();
        assert_eq!(pool.pool.available(), 0);
        assert_eq!(pool.pool.len(), 0);

        let mut obj = pool.get();
        obj.ref_mut_item().value = 20;
        assert_eq!(obj.ref_item().value, 20);
        let item = obj.extract();
        assert_eq!(obj.item, std::ptr::null_mut());
        assert!(item != std::ptr::null_mut());
        unsafe{ assert!((*item).value == 20); }
    }
}