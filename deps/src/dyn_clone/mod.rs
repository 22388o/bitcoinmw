//! Using dyn_clone subject to license agreement here: <https://github.com/dtolnay/dyn-clone/blob/master/LICENSE-MIT>
extern crate alloc;

#[cfg(doc)]
extern crate core as std;

#[macro_use]
mod macros;

// Not public API.
#[doc(hidden)]
pub mod __private {
	#[doc(hidden)]
	pub use core::clone::Clone;
	#[doc(hidden)]
	pub use core::marker::{Send, Sync};

	#[doc(hidden)]
	pub type Box<T> = std::boxed::Box<T>;
}

mod sealed {
	pub trait Sealed {}
	impl<T: Clone> Sealed for T {}
	impl Sealed for str {}
	impl<T: Clone> Sealed for [T] {}
	pub struct Private;
}

use crate::dyn_clone::sealed::{Private, Sealed};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;

/// This trait is implemented by any type that implements [`std::clone::Clone`].
pub trait DynClone: Sealed {
	// Not public API
	#[doc(hidden)]
	fn __clone_box(&self, _: Private) -> *mut ();
}

/// `&T`&ensp;&mdash;&blacktriangleright;&ensp;`T`
pub fn clone<T>(t: &T) -> T
where
	T: DynClone,
{
	unsafe { *Box::from_raw(<T as DynClone>::__clone_box(t, Private) as *mut T) }
}

/// `&T`&ensp;&mdash;&blacktriangleright;&ensp;`Box<T>`
pub fn clone_box<T>(t: &T) -> Box<T>
where
	T: ?Sized + DynClone,
{
	let mut fat_ptr = t as *const T;
	unsafe {
		let data_ptr = &mut fat_ptr as *mut *const T as *mut *mut ();
		assert_eq!(*data_ptr as *const (), t as *const T as *const ());
		*data_ptr = <T as DynClone>::__clone_box(t, Private);
	}
	unsafe { Box::from_raw(fat_ptr as *mut T) }
}

/// `&mut Arc<T>`&ensp;&mdash;&blacktriangleright;&ensp;`&mut T`
pub fn arc_make_mut<T>(arc: &mut Arc<T>) -> &mut T
where
	T: ?Sized + DynClone,
{
	// Atomic. Find out whether the Arc in the argument is the single holder of
	// a reference count (strong or weak) on the target object. If yes, it is
	// guaranteed to remain that way throughout the rest of this function
	// because no other threads could bump the reference count through any other
	// Arc (because no others exist) or through this Arc (because the current
	// thread holds an exclusive borrow of it).
	let is_unique = Arc::get_mut(arc).is_some();
	if !is_unique {
		// Non-atomic.
		let clone = Arc::from(clone_box(&**arc));
		// Atomic. Check the reference counts again to find out whether the old
		// object needs to be dropped. Probably not, but it can happen if all
		// the other holders of a reference count went away during the time that
		// the clone operation took.
		*arc = clone;
	}
	// Non-atomic. TODO: replace with Arc::get_mut_unchecked when stable.
	let ptr = Arc::as_ptr(arc) as *mut T;
	unsafe { &mut *ptr }
}

/// `&mut Rc<T>`&ensp;&mdash;&blacktriangleright;&ensp;`&mut T`
pub fn rc_make_mut<T>(rc: &mut Rc<T>) -> &mut T
where
	T: ?Sized + DynClone,
{
	let is_unique = Rc::get_mut(rc).is_some();
	if !is_unique {
		let clone = Rc::from(clone_box(&**rc));
		*rc = clone;
	}
	let ptr = Rc::as_ptr(rc) as *mut T;
	unsafe { &mut *ptr }
}

impl<T> DynClone for T
where
	T: Clone,
{
	fn __clone_box(&self, _: Private) -> *mut () {
		Box::<T>::into_raw(Box::new(self.clone())) as *mut ()
	}
}

impl DynClone for str {
	fn __clone_box(&self, _: Private) -> *mut () {
		Box::<str>::into_raw(Box::from(self)) as *mut ()
	}
}

impl<T> DynClone for [T]
where
	T: Clone,
{
	fn __clone_box(&self, _: Private) -> *mut () {
		Box::<[T]>::into_raw(self.iter().cloned().collect()) as *mut ()
	}
}
