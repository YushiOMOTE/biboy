use bootloader::bootinfo::{BootInfo, MemoryMap, MemoryRegionType};
use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::{MapToError, OffsetPageTable},
        FrameAllocator, Mapper, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
        UnusedPhysFrame,
    },
    PhysAddr, VirtAddr,
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 10 * 1024 * 1024; // 100 MiB

struct Allocator(usize, &'static MemoryMap);

impl Allocator {
    fn new(map: &'static MemoryMap) -> Self {
        Self(0, map)
    }
}

unsafe impl FrameAllocator<Size4KiB> for Allocator {
    fn allocate_frame(&mut self) -> Option<UnusedPhysFrame<Size4KiB>> {
        let res = self
            .1
            .iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| (r.range.start_addr()..r.range.end_addr()).step_by(4096))
            .flatten()
            .map(|addr| unsafe {
                UnusedPhysFrame::new(PhysFrame::containing_address(PhysAddr::new(addr)))
            })
            .nth(self.0);
        self.0 += 1;
        res
    }
}

pub fn init_heap(boot_info: &'static BootInfo) -> Result<(), MapToError> {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { init(phys_mem_offset) };
    let mut frame_allocator = Allocator::new(&boot_info.memory_map);

    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)?
                .flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}
