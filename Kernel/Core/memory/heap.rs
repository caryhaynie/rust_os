// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
// - Dynamic memory manager

use core::option::{Option,None,Some};
use core::ptr::RawPtr;

// --------------------------------------------------------
// Types
pub enum HeapId
{
	LocalHeap,	// Inaccessible outside of process
	GlobalHeap,	// Global allocations
}

struct HeapDef
{
	start: Option<*mut HeapHead>,
	last_foot: Option<*mut HeapFoot>,
	first_free: Option<*mut HeapHead>,
}

enum HeapState
{
	HeapFree(Option<*mut HeapHead>),
	HeapUsed(uint),
}

struct HeapHead
{
	magic: uint,
	size: uint,
	state: HeapState,
}
struct HeapFoot
{
	head: *mut HeapHead,
}

static MAGIC: uint = 0x71ff11A1;
// --------------------------------------------------------
// Globals
//#[link_section(process_local)] static s_local_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{head:None});
static mut s_global_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{start:None,last_foot:None,first_free:None});

// --------------------------------------------------------
// Code
pub fn init()
{
}

#[lang="exchange_malloc"]
unsafe fn exchange_malloc(size: uint, _align: uint) -> *mut u8 {
	allocate(GlobalHeap, size).unwrap() as *mut u8
}
#[lang="exchange_free"]
unsafe fn exchange_free(ptr: *mut u8, _size: uint, _align: uint) {
	deallocate(ptr)
}

pub unsafe fn alloc<T>(value: T) -> *mut T
{
	let ret = match allocate(GlobalHeap, ::core::mem::size_of::<T>())
		{
		Some(v) => v as *mut T,
		None => fail!("Out of memory")
		};
	::core::ptr::write(ret, value);
	ret
}

pub unsafe fn alloc_array<T>(count: uint) -> *mut T
{
	match allocate(GlobalHeap, ::core::mem::size_of::<T>() * count)
	{
	Some(v) => v as *mut T,
	None => fail!("Out of memory when allocating array of {} elements", count)
	}
}

pub unsafe fn allocate(heap: HeapId, size: uint) -> Option<*mut ()>
{
	match heap
	{
	GlobalHeap => s_global_heap.lock().allocate(size),
	_ => fail!("TODO: Non-global heaps"),
	}
}

//pub unsafe fn expand(pointer: *mut (), newsize: uint) -> Option<*mut ()>
//{
//	fail!("TODO: heap::expand");
//	None
//}

pub unsafe fn deallocate<T>(pointer: *mut T)
{
	s_global_heap.lock().deallocate(pointer as *mut ());
}

impl HeapDef
{
	pub unsafe fn allocate(&mut self, size: uint) -> Option<*mut ()>
	{
		// 1. Round size up to closest heap block size
		let blocksize = ::lib::num::round_up(size + ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>(), 32);
		log_debug!("allocate(size={}) blocksize={}", size, blocksize);
		// 2. Locate a free location
		// Check all free blocks for one that would fit this allocation
		let mut prev = None;
		let mut opt_fb = self.first_free;
		while opt_fb.is_some()
		{
			let fb = &mut *opt_fb.unwrap();
			assert!( fb.magic == MAGIC );
			let next = match fb.state { HeapFree(n)=> n, _ => fail!("Non-free block in free list") };
			if fb.size >= blocksize
			{
				break;
			}
			prev = opt_fb;
			opt_fb = next;
		}
		if opt_fb.is_some()
		{
			let fb = &mut *opt_fb.unwrap();
			let next = match fb.state { HeapFree(n)=> n, _ => fail!("Non-free block in free list") };
			//log_trace!("allocate - Suitable free block {}!", fb as *mut _);
			// Split block (if needed)
			if fb.size > blocksize
			{
				let far_foot = fb.foot() as *mut _;
				let far_size = fb.size - blocksize;
				fb.size = blocksize;
				*fb.foot() = HeapFoot {
					head: fb as *mut _,
					};
				let far_head = fb.next();
				//log_trace!("Creating new block at {}", far_head);
				*far_head = HeapHead {
					magic: MAGIC,
					size: far_size,
					state: HeapFree(next)
					};
				(*far_foot).head = far_head;
				match prev
				{
				Some(x) => {(*x).state = HeapFree(Some(far_head));},
				None => {self.first_free = Some(far_head);},
				}
			}
			// Return newly allocated block
			fb.state = HeapUsed(size);
			log_trace!("Returning {} (was free)", fb.data());
			return Some( fb.data() );
		}
		// Fall: No free blocks would fit the allocation
		//log_trace!("allocate - No suitable free blocks");
		
		// 3. If none, allocate more space
		let block_ptr = self.expand(blocksize);
		let block = &mut *block_ptr;
		// > Split returned block into a block of required size and a free block
		if block.size > blocksize
		{
			// Create a new block header at end of block
			let tailsize = block.size - blocksize;
			block.size = blocksize;
			*block.foot() = HeapFoot {
				head: block_ptr,
				};
			let tailblock = &mut *block.next();
			*tailblock = HeapHead {
				magic: MAGIC,
				size: tailsize,
				state: HeapFree(self.first_free),
				};
			tailblock.foot().head = block.next();
			self.first_free = Some(block.next());
		}
		
		log_trace!("Returning {} (new)", block.data());
		Some( block.data() )
	}

	pub fn deallocate(&mut self, ptr: *mut ())
	{
		log_debug!("deallocate(ptr={})", ptr);
		unsafe
		{
			let headptr = (ptr as *mut HeapHead).offset(-1);
			assert!( (*headptr).magic == MAGIC );
			assert!( (*headptr).foot().head() as *mut _ == headptr );
			
			(*headptr).state = HeapFree(self.first_free);
			self.first_free = Some( headptr );
		}
	}
	
	/// Expand the heap to create a block at least `min_size` bytes long at the end
	/// \return New block, pre-allocated
	unsafe fn expand(&mut self, min_size: uint) -> *mut HeapHead
	{
		let use_prev =
			if self.start.is_none() {
				let base = ::arch::memory::addresses::heap_start;
				self.start = Some( base as *mut HeapHead );
				self.last_foot = Some( (base as *mut HeapFoot).offset(-1) );
				false
			}
			else {
				match (*self.last_foot.unwrap()).head().state
				{
				HeapFree(_) => true,
				HeapUsed(_) => false
				}
			};
		let last_foot = &mut *self.last_foot.unwrap();
		let alloc_size = min_size - (if use_prev { last_foot.head().size } else { 0 });
		
		// 1. Allocate at least one page at the end of the heap
		let n_pages = ::lib::num::round_up(alloc_size, ::PAGE_SIZE) / ::PAGE_SIZE;
		log_debug!("HeapDef.expand(min_size={}), n_pages={}", min_size, n_pages);
		assert!(n_pages > 0);
		::memory::virt::allocate(last_foot.next_head() as *mut(), n_pages);
		
		// 2. If the final block is a free block, allocate it and expand to cover the new area
		let block = if use_prev
			{
				let block = &mut *last_foot.head;
				log_debug!("HeapDef.expand: (prev) &block={}", block as *mut HeapHead);
				block.size += n_pages * ::PAGE_SIZE;
				block.foot().head = last_foot.head;
				
				block
			}
			else
			{
				let block = &mut *last_foot.next_head();
				log_debug!("HeapDef.expand: (new) &block={}", block as *mut HeapHead);
				*block = HeapHead {
					magic: MAGIC,
					state: HeapUsed(0),
					size: n_pages * ::PAGE_SIZE,
					};
				block.foot().head = last_foot.next_head();
				
				block
			};
		self.last_foot = Some(block.foot() as *mut HeapFoot);
		log_debug!("HeapDef.expand: &block={}", block as *mut HeapHead);
		block.state = HeapUsed(0);
		// 3. Return final block
		block
	}
}

impl HeapHead
{
	unsafe fn ptr(&self) -> *mut HeapHead { ::core::mem::transmute(self) }
	pub unsafe fn next(&self) -> *mut HeapHead
	{
		(self.ptr() as *mut u8).offset( self.size as int ) as *mut HeapHead
	}
	pub unsafe fn data(&mut self) -> *mut ()
	{
		self.ptr().offset( 1 ) as *mut ()
	}
	pub unsafe fn foot<'a>(&'a self) -> &'a mut HeapFoot
	{
		::core::mem::transmute( (self.next() as *mut HeapFoot).offset( -1 ) )
	}
}

impl HeapFoot
{
	pub unsafe fn head<'a>(&'a mut self) -> &'a mut HeapHead
	{
		::core::mem::transmute( self.head )
	}
	pub unsafe fn next_head(&mut self) -> *mut HeapHead
	{
		let self_ptr: *mut HeapFoot = ::core::mem::transmute(self);
		self_ptr.offset(1) as *mut HeapHead
	}
}

// vim: ft=rust
