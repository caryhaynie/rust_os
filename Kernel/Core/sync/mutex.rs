//
//
//
use lib::LazyStatic;
use core::marker::{Send, Sync};
use core::ops::Fn;

/// A standard mutex
pub struct Mutex<T: Send>
{
	pub locked_held: ::sync::Spinlock<bool>,
	pub queue: ::core::cell::UnsafeCell<::threads::WaitQueue>,
	pub val: ::core::cell::UnsafeCell<T>,
}
// Mutexes are inherently sync
unsafe impl<T: Send> Sync for Mutex<T> { }

/// Lock handle on a mutex
struct HeldMutex<'lock,T:'lock+Send>
{
	lock: &'lock Mutex<T>
}

/// A lazily populated mutex (contained type is allocated on the heap upon first lock)
pub struct LazyMutex<T: Send>(pub Mutex<LazyStatic<T>>);

impl<T: Send> Mutex<T>
{
	/*
	pub fn new(val: T) -> Mutex<T> {
		Mutex {
			locked_held: spinlock_init!(false),
			queue: ::threads::WAITQUEUE_INIT,
			val: val,
		}
	}
	*/
	
	/// Lock the mutex
	pub fn lock(&self) -> HeldMutex<T> {
		{
			// Check the held status of the mutex
			// - Spinlock protected variable
			let mut held = self.locked_held.lock();
			if *held != false
			{
				// If mutex is locked, then wait for it to be unlocked
				// - ThreadList::wait will release the passed spinlock
				unsafe { (*self.queue.get()).wait(held) };
			}
			else
			{
				*held = true;
			}
		}
		return HeldMutex { lock: self };
	}
	/// Release the mutex
	fn unlock(&self) {
		let mut held = self.locked_held.lock();
		*held = false;
		unsafe { (*self.queue.get()).wake_one() };
		// TODO: Wake anything waiting
	}
}

impl<T: Send> LazyMutex<T>
{
	/// Lock and (if required) initialise using init_fcn
	pub fn lock<Fcn: Fn()->T>(&self, init_fcn: Fcn) -> HeldMutex<LazyStatic<T>>
	{
		let mut lh = self.0.lock();
		lh.prep(init_fcn);
		lh
	}
}

#[unsafe_destructor]
impl<'lock,T:Send> ::core::ops::Drop for HeldMutex<'lock,T>
{
	/// Unlock on drop of HeldMutex
	fn drop(&mut self) {
		self.lock.unlock();
	}
}
impl<'lock,T:Send> ::core::ops::Deref for HeldMutex<'lock,T>
{
	type Target = T;
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T:Send> ::core::ops::DerefMut for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.val.get() }
	}
}

#[macro_export]
macro_rules! mutex_init{ ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
	val: ::core::cell::UnsafeCell{ value: $val },
	}) }
macro_rules! lazymutex_init{
	() => {::sync::mutex::LazyMutex(mutex_init!( ::lib::LazyStatic(None) ))}
}

// vim: ft=rust

