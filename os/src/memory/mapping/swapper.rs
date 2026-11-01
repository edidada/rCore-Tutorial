//! 页面置换算法

use super::*;
use crate::memory::{frame::FrameTracker, *};
use alloc::collections::VecDeque;

/// 管理一个线程所映射的页面的置换操作
pub trait Swapper {
    /// 新建带有一个分配数量上限的置换器
    fn new(quota: usize) -> Self;

    /// 是否已达到上限
    fn full(&self) -> bool;

    /// 取出一组映射
    fn pop(&mut self) -> Option<(VirtualPageNumber, FrameTracker)>;

    /// 添加一组映射（不会在以达到分配上限时调用）
    fn push(&mut self, vpn: VirtualPageNumber, frame: FrameTracker, entry: *mut PageTableEntry);

    /// 只保留符合某种条件的条目（用于移除一段虚拟地址）
    fn retain(&mut self, predicate: impl Fn(&VirtualPageNumber) -> bool);
}

pub type SwapperImpl = ClockSwapper;

/// 页面置换算法基础实现：FIFO
pub struct FIFOSwapper {
    /// 记录映射和添加的顺序
    queue: VecDeque<(VirtualPageNumber, FrameTracker)>,
    /// 映射数量上限
    quota: usize,
}

impl Swapper for FIFOSwapper {
    fn new(quota: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            quota,
        }
    }
    fn full(&self) -> bool {
        self.queue.len() == self.quota
    }
    fn pop(&mut self) -> Option<(VirtualPageNumber, FrameTracker)> {
        self.queue.pop_front()
    }
    fn push(&mut self, vpn: VirtualPageNumber, frame: FrameTracker, _entry: *mut PageTableEntry) {
        self.queue.push_back((vpn, frame));
    }
    fn retain(&mut self, predicate: impl Fn(&VirtualPageNumber) -> bool) {
        self.queue.retain(|(vpn, _)| predicate(vpn));
    }
}

/// 二次机会 Clock：队首是时钟指针，命中 A 位则清除并移动到队尾。
pub struct ClockSwapper {
    queue: VecDeque<(VirtualPageNumber, FrameTracker, usize)>,
    quota: usize,
}

impl Swapper for ClockSwapper {
    fn new(quota: usize) -> Self {
        assert!(quota > 0);
        Self { queue: VecDeque::new(), quota }
    }
    fn full(&self) -> bool { self.queue.len() >= self.quota }
    fn pop(&mut self) -> Option<(VirtualPageNumber, FrameTracker)> {
        loop {
            let (vpn, frame, address) = self.queue.pop_front()?;
            // 页表帧由 Mapping 持有；交换器删除这些记录前页表始终存在。
            let pointer = address as *mut PageTableEntry;
            let mut entry = unsafe { core::ptr::read_volatile(pointer) };
            if entry.flags().contains(Flags::ACCESSED) {
                entry.set_flags(entry.flags() - Flags::ACCESSED);
                unsafe {
                    core::ptr::write_volatile(pointer, entry);
                    // 清 A 位后刷新 TLB，后续硬件访问才能重新设置 A。
                    llvm_asm!("sfence.vma" :::: "volatile");
                }
                self.queue.push_back((vpn, frame, address));
            } else {
                return Some((vpn, frame));
            }
        }
    }
    fn push(&mut self, vpn: VirtualPageNumber, frame: FrameTracker, entry: *mut PageTableEntry) {
        assert!(!self.full());
        assert!(!entry.is_null());
        self.queue.push_back((vpn, frame, entry as usize));
    }
    fn retain(&mut self, predicate: impl Fn(&VirtualPageNumber) -> bool) {
        self.queue.retain(|(vpn, _, _)| predicate(vpn));
    }
}

/// 以真实页表项验证第二次机会、全 A 位扫描和 retain。
pub fn clock_self_test() {
    use alloc::boxed::Box;
    let mut clock = ClockSwapper::new(3);
    let mut entries = alloc::vec::Vec::new();
    for i in 0..3 {
        let frame = FRAME_ALLOCATOR.lock().alloc().unwrap();
        let mut entry = Box::new(PageTableEntry::new(Some(frame.page_number()),
            Flags::READABLE | if i == 0 { Flags::ACCESSED } else { Flags::empty() }));
        clock.push(VirtualPageNumber(i), frame, &mut *entry);
        entries.push(entry);
    }
    assert!(clock.full());
    assert_eq!(clock.pop().unwrap().0, VirtualPageNumber(1));
    assert!(!entries[0].flags().contains(Flags::ACCESSED));
    assert_eq!(clock.pop().unwrap().0, VirtualPageNumber(2));
    let accessed = entries[0].flags() | Flags::ACCESSED;
    entries[0].set_flags(accessed);
    assert_eq!(clock.pop().unwrap().0, VirtualPageNumber(0));
    assert!(clock.pop().is_none());
    let frame = FRAME_ALLOCATOR.lock().alloc().unwrap();
    clock.push(VirtualPageNumber(7), frame, &mut *entries[0]);
    clock.retain(|_| false);
    assert!(clock.pop().is_none());
    println!("Clock policy tests passed");
}
