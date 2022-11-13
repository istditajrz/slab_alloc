use core::alloc;
use core::sync::atomic::{self, Ordering};

/// Result type for allocation errors
pub type Result<T> = core::result::Result<T, alloc::AllocError>;

/// Possible sizes of sections
pub enum Atomics {
    /// One block
    Bool(atomic::AtomicBool),
    /// 8 blocks
    U8(atomic::AtomicU8),
    /// 16 blocks
    U16(atomic::AtomicU16),
    /// 32 blocks
    U32(atomic::AtomicU32),
    /// 64 blocks
    U64(atomic::AtomicU64),
}

macro_rules! from_atomic {
    (impl From<$(($atomic:ty, $variant:path)),+> for Atomics;) => {
        $(
            impl From<$atomic> for Atomics {
                fn from(t: $atomic) -> Self {
                    $variant(t)
                }
            }
        )+
    };
}

from_atomic! {
    impl From<
        (atomic::AtomicBool, Atomics::Bool),
        (atomic::AtomicU8, Atomics::U8),
        (atomic::AtomicU16, Atomics::U16),
        (atomic::AtomicU32, Atomics::U32),
        (atomic::AtomicU64, Atomics::U64)
    > for Atomics;
}

/// A struct that describes how large slabs should be and the quantity
pub struct Section {
    /// The size of the slabs
    pub size: usize,
    pub(crate) allocated: Atomics,
}

impl Section {
    /// Constructor of section
    pub fn new<A: Into<Atomics>>(size: usize, quantity: A) -> Self {
        Self {
            size,
            allocated: quantity.into(),
        }
    }

    pub(crate) fn allocate(&self) -> Result<u32> {
        // Abstracted (don't want to copy it 4 times):
        //
        //  // Acquire current value
        //  let load = u.load(Ordering::Acquire);
        //
        //  // Check if there are any free slots
        //  if !load == 0 {
        //      Err(alloc::AllocError)
        //  } else {
        //
        //      // Shamelessly stolen from: https://stackoverflow.com/questions/31393100/how-to-get-position-of-right-most-set-bit-in-c
        //      let set_bit = !load & (load + 1);
        //
        //      // Set bit to be allocated (with paired release)
        //      u.store(load | set_bit, Ordering::Release);
        //
        //      // Return index
        //      Ok(set_bit.trailing_zeros())
        //  }
        match &self.allocated {
            Atomics::Bool(b) => {
                match b.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed) {
                    Ok(false) => Ok(0),
                    _ => Err(alloc::AllocError),
                }
            }
            Atomics::U8(u) => {
                let load = u.load(Ordering::Acquire);
                if !load == 0 {
                    Err(alloc::AllocError)
                } else {
                    let set_bit = !load & (load + 1);
                    u.store(load | set_bit, Ordering::Release);
                    Ok(set_bit.trailing_zeros())
                }
            }
            Atomics::U16(u) => {
                let load = u.load(Ordering::Acquire);
                if !load == 0 {
                    Err(alloc::AllocError)
                } else {
                    let set_bit = !load & (load + 1);
                    u.store(load | set_bit, Ordering::Release);
                    Ok(set_bit.trailing_zeros())
                }
            }
            Atomics::U32(u) => {
                let load = u.load(Ordering::Acquire);
                if !load == 0 {
                    Err(alloc::AllocError)
                } else {
                    let set_bit = !load & (load + 1);
                    u.store(load | set_bit, Ordering::Release);
                    Ok(set_bit.trailing_zeros())
                }
            }
            Atomics::U64(u) => {
                let load = u.load(Ordering::Acquire);
                if !load == 0 {
                    Err(alloc::AllocError)
                } else {
                    let set_bit = !load & (load + 1);
                    u.store(load | set_bit, Ordering::Release);
                    Ok(set_bit.trailing_zeros())
                }
            }
        }
    }

    pub(crate) fn deallocate(&self, index: u32) -> Result<()> {
        match &self.allocated {
            Atomics::Bool(b) => {
                match b.compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed) {
                    Ok(true) => Ok(()),
                    _ => Err(alloc::AllocError),
                }
            }
            Atomics::U8(u) => {
                let load = u.load(Ordering::Acquire);
                let set_bit = 1u8 << index;
                if load & set_bit == 0 {
                    Err(alloc::AllocError)
                } else {
                    u.store(load & !set_bit, Ordering::Release);
                    Ok(())
                }
            }
            Atomics::U16(u) => {
                let load = u.load(Ordering::Acquire);
                let set_bit = 1u16 << index;
                if load & set_bit == 0 {
                    Err(alloc::AllocError)
                } else {
                    u.store(load & !set_bit, Ordering::Release);
                    Ok(())
                }
            }
            Atomics::U32(u) => {
                let load = u.load(Ordering::Acquire);
                let set_bit = 1u32 << index;
                if load & set_bit == 0 {
                    Err(alloc::AllocError)
                } else {
                    u.store(load & !set_bit, Ordering::Release);
                    Ok(())
                }
            }
            Atomics::U64(u) => {
                let load = u.load(Ordering::Acquire);
                let set_bit = 1u64 << index;
                if load & set_bit == 0 {
                    Err(alloc::AllocError)
                } else {
                    u.store(load & !set_bit, Ordering::Release);
                    Ok(())
                }
            }
        }
    }

    /// The amount of slots unallocated
    pub fn free_slots(&self) -> u32 {
        match &self.allocated {
            Atomics::Bool(u) => u32::from(!u.load(Ordering::Relaxed)),
            Atomics::U8(u) => u.load(Ordering::Relaxed).count_zeros(),
            Atomics::U16(u) => u.load(Ordering::Relaxed).count_zeros(),
            Atomics::U32(u) => u.load(Ordering::Relaxed).count_zeros(),
            Atomics::U64(u) => u.load(Ordering::Relaxed).count_zeros(),
        }
    }

    /// The total number of slots available
    pub fn total_slots(&self) -> u32 {
        match &self.allocated {
            Atomics::Bool(_) => 1,
            Atomics::U8(_) => 8,
            Atomics::U16(_) => 16,
            Atomics::U32(_) => 32,
            Atomics::U64(_) => 64,
        }
    }

    /// The percent of the section is unallocated
    pub fn percent_free(&self) -> f32 {
        (self.free_slots() as f32 / self.total_slots() as f32) * 100.0
    }
}

#[cfg(test)]
mod test {
    macro_rules! tests {
        ($(($alloc_fun_name:ident, $dealloc_fun_name:ident, $num_type:ty, $atomic_type:ty)),+) => {
            $(
                #[test]
                fn $alloc_fun_name() {
                    use crate::section::*;
                    let section: Section = Section::new(0, <$atomic_type>::new(0));
                    for _ in 0..<$num_type>::BITS {
                        assert!(section.allocate().is_ok());
                    }
                    assert!(section.allocate().is_err());
                    assert!(section.free_slots() == 0);
                }

                #[test]
                fn $dealloc_fun_name() {
                    use crate::section::*;
                    let section: Section = Section::new(0, <$atomic_type>::new(<$num_type>::MAX));
                    for i in 0..<$num_type>::BITS {
                        assert!(section.deallocate(i).is_ok());
                    }
                    for i in 0..<$num_type>::BITS {
                        assert!(section.deallocate(i).is_err());
                    }
                    assert!(section.free_slots() == <$num_type>::BITS);
                }
            )+
        };
    }
    tests! {
        (u8_alloc, u8_dealloc, u8, atomic::AtomicU8),
        (u16_alloc, u16_dealloc, u16, atomic::AtomicU16),
        (u32_alloc, u32_dealloc, u32, atomic::AtomicU32),
        (u64_alloc, u64_dealloc, u64, atomic::AtomicU64)
    }

    #[test]
    fn bool_alloc() {
        use crate::section::*;
        let section: Section = Section::new(0, atomic::AtomicBool::new(false));
        assert!(section.allocate().is_ok());
        assert!(section.allocate().is_err());
        assert!(section.free_slots() == 0);
    }

    #[test]
    fn bool_dealloc() {
        use crate::section::*;
        let section: Section = Section::new(0, atomic::AtomicBool::new(true));
        assert!(section.deallocate(0).is_ok());
        assert!(section.deallocate(0).is_err());
        assert!(section.free_slots() == 1);
    }
}
