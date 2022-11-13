#![feature(allocator_api)]
#![feature(error_in_core)]
#![warn(missing_docs)]
#![no_std]

//! A library that implements the [Slab Allocator](https://en.wikipedia.org/wiki/Slab_allocation) using
//! the rust [allocator_api](https://github.com/rust-lang/rust/issues/32838) ([repo](https://github.com/rust-lang/wg-allocators))

/// Types to describe allocation states of slab sizes
pub mod section;
use core::alloc;
pub use section::{Atomics, Section};

/// The main struct which encapsulates the allocator.
/// 'm is the lifetime of the buffer passed and
/// const N is the number of different slab sizes
pub struct SlabAllocator<'m, const N: usize> {
    pub(crate) blocks: [Section; N],
    pub(crate) buffer: [&'m [u8]; N],
}

/// Error returned during creation of a [`SlabAllocator`] if the buffer passed is too small
#[derive(Debug, Clone, Copy)]
pub struct BufTooSmall;

impl core::fmt::Display for BufTooSmall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "BufTooSmall")
    }
}

impl core::error::Error for BufTooSmall {}

impl<'m, const N: usize> SlabAllocator<'m, N> {
    /// Constructor for [`SlabAllocator`] where
    /// `blocks` are the number, sizes and capacity of blocks passed to the allocator and
    /// `buf` is the memory buffer that the allocator will allocate from
    pub fn new(
        blocks: [Section; N],
        mut buf: &'m mut [u8],
    ) -> core::result::Result<Self, BufTooSmall> {
        let mut buffer: [&'m [u8]; N] = [&[]; N];
        for (index, section) in blocks.iter().enumerate() {
            let size = match section.allocated {
                Atomics::Bool(_) => section.size,
                Atomics::U8(_) => 8 * section.size,
                Atomics::U16(_) => 16 * section.size,
                Atomics::U32(_) => 32 * section.size,
                Atomics::U64(_) => 64 * section.size,
            };
            if size > buf.len() {
                return Err(BufTooSmall);
            }
            let (section_block, rest) = buf.split_at_mut(size);
            buf = rest;
            buffer[index] = section_block;
        }
        Ok(Self { blocks, buffer })
    }

    /// The percentage of the capacity that is free for each section
    pub fn percent_free(&self) -> [f32; N] {
        let mut out = [0.0; N];
        out.iter_mut()
            .zip(self.blocks.iter())
            .for_each(|(arr, section)| *arr = section.percent_free());
        out
    }
}

unsafe impl<'m, const N: usize> alloc::Allocator for SlabAllocator<'m, N> {
    fn allocate(&self, layout: alloc::Layout) -> Result<ptr::NonNull<[u8]>, alloc::AllocError> {
        // Target size of block
        let size = layout.pad_to_align().size();

        // Find the smallest size section larger than the target size
        let (index, section) = self
            .blocks
            .iter()
            .enumerate()
            .find(|(_, section)| section.size >= size && section.free_slots() > 0)
            .ok_or(alloc::AllocError)?;

        // Calculate the offset within the section and mark it as allocated
        let offset = section.allocate()? as usize;

        Ok(self.buffer[index][offset..(offset + section.size)].into())
    }
    unsafe fn deallocate(&self, ptr: ptr::NonNull<u8>, _layout: alloc::Layout) {
        // Find section allocated in
        let (index, buffer) = self
            .buffer
            .iter()
            .enumerate()
            .find(|(_, s)| s.as_ptr_range().contains(&(ptr.as_ptr() as *const _)))
            .expect("Could not deallocate slab: could not find section ptr is allocated in");

        // Calculate byte offset in the section
        let offset = ptr.as_ptr().offset_from(buffer.as_ptr()) as u32;

        // Deallocate the block
        self.blocks[index]
            .deallocate(offset)
            .expect("Could not deallocate block");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::{alloc::Allocator, sync::atomic::*};

    #[test]
    fn initialise() {
        extern crate std;
        let mut small_buf = [0u8; 10];
        assert!(
            SlabAllocator::new([Section::new(100, AtomicU8::new(0))], &mut small_buf[..]).is_err()
        );

        let mut large_buf = [0u8; 1024];
        assert!(
            SlabAllocator::new([Section::new(100, AtomicU8::new(0))], &mut large_buf[..]).is_ok()
        );

        let mut exact_buf = [0u8; 800];
        assert!(
            SlabAllocator::new([Section::new(100, AtomicU8::new(0))], &mut exact_buf[..]).is_ok()
        );
    }

    #[test]
    fn boxes() {
        extern crate std;
        let mut buf = [0u8; 1024];
        let allocator = SlabAllocator::new(
            [Section::new(
                std::mem::size_of::<std::boxed::Box<u32>>(),
                AtomicU64::new(0),
            )],
            &mut buf[..],
        )
        .expect("Creation of allocator failed");

        let mut b = std::boxed::Box::new_in(0, allocator.by_ref());
        for i in 0..u64::BITS {
            b = std::boxed::Box::new_in(i, allocator.by_ref());
        }
        assert_eq!(*b, 63);
    }
}
