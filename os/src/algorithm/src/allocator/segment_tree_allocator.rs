//! 叶子记录单页空闲状态，内部节点记录区间空闲页数。
use super::Allocator;
use alloc::{vec, vec::Vec};

pub struct SegmentTreeAllocator {
    capacity: usize,
    base: usize,
    free: Vec<usize>,
}

impl SegmentTreeAllocator {
    fn update(&mut self, index: usize, value: usize) {
        let mut node = self.base + index;
        self.free[node] = value;
        while node > 1 {
            node /= 2;
            self.free[node] = self.free[node * 2] + self.free[node * 2 + 1];
        }
    }
}

impl Allocator for SegmentTreeAllocator {
    fn new(capacity: usize) -> Self {
        let base = capacity.max(1).next_power_of_two();
        let mut free = vec![0; base * 2];
        for leaf in &mut free[base..base + capacity] {
            *leaf = 1;
        }
        for node in (1..base).rev() {
            free[node] = free[node * 2] + free[node * 2 + 1];
        }
        Self { capacity, base, free }
    }

    fn alloc(&mut self) -> Option<usize> {
        if self.free[1] == 0 {
            return None;
        }
        let mut node = 1;
        while node < self.base {
            node *= 2;
            if self.free[node] == 0 {
                node += 1;
            }
        }
        let index = node - self.base;
        self.update(index, 0);
        Some(index)
    }

    fn dealloc(&mut self, index: usize) {
        assert!(index < self.capacity, "frame index out of range");
        assert_eq!(self.free[self.base + index], 0, "double free");
        self.update(index, 1);
    }
}

/// 同一组断言同时在宿主机单测及 RISC-V 内核执行。
pub fn segment_tree_self_test() {
    for &capacity in &[0, 1, 3, 17, 64, 127] {
        let mut allocator = SegmentTreeAllocator::new(capacity);
        for index in 0..capacity {
            assert_eq!(allocator.alloc(), Some(index));
        }
        assert_eq!(allocator.alloc(), None);
        for index in (0..capacity).rev() {
            allocator.dealloc(index);
        }
        for index in 0..capacity {
            assert_eq!(allocator.alloc(), Some(index));
        }
        assert_eq!(allocator.alloc(), None);
        if capacity > 0 {
            allocator.dealloc(capacity / 2);
            assert_eq!(allocator.alloc(), Some(capacity / 2));
            assert_eq!(allocator.alloc(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhaustion_and_recycling() { segment_tree_self_test(); }
    #[test]
    #[should_panic(expected = "double free")]
    fn reject_double_free() { SegmentTreeAllocator::new(3).dealloc(1); }
    #[test]
    #[should_panic(expected = "frame index out of range")]
    fn reject_out_of_range() { SegmentTreeAllocator::new(3).dealloc(3); }
}
