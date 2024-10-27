use std::{cell::UnsafeCell, sync::atomic::AtomicBool};
use std::sync::atomic::Ordering;

struct Spinlock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

struct Guard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<T> Spinlock<T> {

    pub fn new(data: T) -> Self {
        Spinlock {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(data)
        }
    }
    pub fn lock(&self) -> Guard<T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        Guard { lock: self }
    }

}

unsafe impl<T> Send for Guard<'_, T> where T: Send {}
unsafe impl<T> Sync for Guard<'_, T> where T: Sync {}


use std::ops::{Deref, DerefMut};

impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: The very existence of this Guard
        // guarantees we've exclusively locked the lock.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

unsafe impl<T> Sync for Spinlock<T> where T: Send {}


fn main() {
    let x = Spinlock::new(Vec::new());
    let x = std::sync::Arc::new(x); // Wrap the Spinlock in an Arc to share it across threads safely
    std::thread::scope(|s| {
        let x_clone = x.clone();
        s.spawn(move || x_clone.lock().push(1));
        let x_clone = x.clone();
        s.spawn(move || {
            let mut g = x_clone.lock();
            g.push(2);
            g.push(2);
        });
    });
    let g = x.lock();
    print!("{:?}", g.as_slice());
    assert!(g.as_slice() == [1, 2, 2] || g.as_slice() == [2, 2, 1]);
}
