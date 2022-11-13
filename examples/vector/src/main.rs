#![feature(allocator_api)]

use slab::SlabAllocator;
use std::alloc::Allocator;

fn main() {
    // Create buffer to allocate into
    let mut buf = [0u8; 80];

    // Create allocator to allocate with
    let allocator = SlabAllocator::new(
        // 8 slabs that are 10 bytes in size
        [slab::Section::new(10, std::sync::atomic::AtomicU8::new(0))],
        &mut buf[..],
    )
    .unwrap();

    // Create vector
    let mut new: Vec<u8, _> = Vec::with_capacity_in(10, allocator.by_ref());
    // -- do work with vec --
    for i in 0..10 {
        new.push(i);
    }
    println!("{:?}", new);
}
