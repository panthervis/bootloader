use core::fmt;
use core::ops::{Deref, DerefMut};

const PAGE_SIZE: u64 = 4096;

const MAX_MEMORY_MAP_SIZE: usize = 64;

#[repr(C)]
pub struct MemoryMap {
    entries: [MemoryRegion; MAX_MEMORY_MAP_SIZE],
    // u64 instead of usize so that the structure layout is platform
    // independent
    next_entry_index: u64,
}

impl MemoryMap {
    pub fn new() -> Self {
        MemoryMap {
            entries: [MemoryRegion::empty(); MAX_MEMORY_MAP_SIZE],
            next_entry_index: 0,
        }
    }

    pub fn add_region(&mut self, region: MemoryRegion) {
        assert!(self.next_entry_index() < MAX_MEMORY_MAP_SIZE,
            "too many memory regions in memory map");
        self.entries[self.next_entry_index()] = region;
        self.next_entry_index += 1;
        self.sort();
    }

    pub fn sort(&mut self) {
        use core::cmp::Ordering;

        self.entries.sort_unstable_by(|r1, r2| {
            if r1.range.is_empty() {
                Ordering::Greater
            } else if r2.range.is_empty() {
                Ordering::Less
            } else {
                let ordering = r1
                    .range
                    .start_frame_number
                    .cmp(&r2.range.start_frame_number);

                if ordering == Ordering::Equal {
                    r1.range.end_frame_number.cmp(&r2.range.end_frame_number)
                } else {
                    ordering
                }
            }
        });
        if let Some(first_zero_index) = self.entries.iter().position(|r| r.range.is_empty()) {
            self.next_entry_index = first_zero_index as u64;
        }
    }

    fn next_entry_index(&self) -> usize {
        self.next_entry_index as usize
    }
}

impl Deref for MemoryMap {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        &self.entries[0..self.next_entry_index()]
    }
}

impl DerefMut for MemoryMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let next_index = self.next_entry_index();
        &mut self.entries[0..next_index]
    }
}

impl fmt::Debug for MemoryMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct MemoryRegion {
    pub range: FrameRange,
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    pub fn empty() -> Self {
        MemoryRegion {
            range: FrameRange {
                start_frame_number: 0,
                end_frame_number: 0,
            },
            region_type: MemoryRegionType::Empty,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct FrameRange {
    pub start_frame_number: u64,
    // exclusive
    pub end_frame_number: u64,
}

impl FrameRange {
    /// Create a new FrameRange from the passed start_addr and end_addr.
    ///
    /// The end_addr is exclusive.
    pub fn new(start_addr: u64, end_addr: u64) -> Self {
        let last_byte = end_addr - 1;
        FrameRange {
            start_frame_number: start_addr / PAGE_SIZE,
            end_frame_number: (last_byte / PAGE_SIZE) + 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start_frame_number == self.end_frame_number
    }

    pub fn start_addr(&self) -> u64 {
        self.start_frame_number * PAGE_SIZE
    }

    pub fn end_addr(&self) -> u64 {
        self.end_frame_number * PAGE_SIZE
    }
}

impl fmt::Debug for FrameRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "FrameRange({:#x}..{:#x})",
            self.start_addr(),
            self.end_addr()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum MemoryRegionType {
    /// free RAM
    Usable,
    /// used RAM
    InUse,
    /// unusable
    Reserved,
    /// ACPI reclaimable memory
    AcpiReclaimable,
    /// ACPI NVS memory
    AcpiNvs,
    /// Area containing bad memory
    BadMemory,
    /// kernel memory
    Kernel,
    /// kernel stack memory
    KernelStack,
    /// memory used by page tables
    PageTable,
    /// memory used by the bootloader
    Bootloader,
    /// frame at address zero
    ///
    /// (shouldn't be used because it's easy to make mistakes related to null pointers)
    FrameZero,
    /// an empty region with size 0
    Empty,
    /// used for storing the boot information
    BootInfo,
    /// used for storing the supplied package
    Package,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct E820MemoryRegion {
    pub start_addr: u64,
    pub len: u64,
    pub region_type: u32,
    pub acpi_extended_attributes: u32,
}

impl From<E820MemoryRegion> for MemoryRegion {
    fn from(region: E820MemoryRegion) -> MemoryRegion {
        let region_type = match region.region_type {
            1 => MemoryRegionType::Usable,
            2 => MemoryRegionType::Reserved,
            3 => MemoryRegionType::AcpiReclaimable,
            4 => MemoryRegionType::AcpiNvs,
            5 => MemoryRegionType::BadMemory,
            t => panic!("invalid region type {}", t),
        };
        MemoryRegion {
            range: FrameRange::new(region.start_addr, region.start_addr + region.len),
            region_type,
        }
    }
}

extern "C" {
    fn _improper_ctypes_check(_boot_info: MemoryMap);
}
