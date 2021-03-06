// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/_common.rs
// - Common global definitons
//
// All files have 'use _common::*' as the first line, which imports the names from this module
pub use core::iter::{range,range_step};
pub use core::iter::{Iterator,IteratorExt,DoubleEndedIterator};
pub use core::slice::{AsSlice,SliceExt};
pub use core::str::{Str,StrExt};
pub use core::default::Default;
pub use core::option::Option::{self,Some,None};
pub use core::result::Result::{self,Ok,Err};
pub use core::ops::{Drop,Deref,DerefMut,Fn};
pub use core::cmp::PartialEq;
pub use core::num::Int;
pub use core::marker::{Send,Copy};
pub use core::any::Any;
pub use core::ptr::PtrExt;

pub use lib::mem::Box;
pub use lib::vec::Vec;
pub use lib::string::String;
pub use lib::clone::Clone;
pub use lib::collections::{MutableSeq};
pub use lib::{OptPtr,OptMutPtr};

pub use logging::HexDump;

// vim: ft=rust
