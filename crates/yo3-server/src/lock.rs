//! Locks that survive a panic. A panic while a lock is held poisons it; with `.lock().expect(..)` every later user
//! (the single clock task, the accept loops) would panic too, and one bug would end the whole server. The state
//! behind these locks is plain data kept consistent between statements, so the guard is taken regardless.
use std::sync::{Mutex, MutexGuard, PoisonError};

pub trait Locked<T> {
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
