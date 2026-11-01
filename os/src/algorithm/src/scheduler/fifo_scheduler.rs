//! 先入先出队列的调度器 [`FifoScheduler`]

use super::Scheduler;
use alloc::vec::Vec;

/// 采用 FIFO 算法的线程调度器
pub struct FifoScheduler<ThreadType: Clone + Eq> {
    pool: Vec<ThreadType>,
}

/// `Default` 创建一个空的调度器
impl<ThreadType: Clone + Eq> Default for FifoScheduler<ThreadType> {
    fn default() -> Self {
        Self {
            pool: Vec::new(),
        }
    }
}

impl<ThreadType: Clone + Eq> Scheduler<ThreadType> for FifoScheduler<ThreadType> {
    type Priority = ();
    fn add_thread(&mut self, thread: ThreadType) {
        // 加入链表尾部
        self.pool.push(thread);
    }
    fn get_next(&mut self) -> Option<ThreadType> {
        // 从头部取出放回尾部，同时将其返回
        if !self.pool.is_empty() {
            let thread = self.pool.remove(0);
            self.pool.push(thread.clone());
            Some(thread)
        } else {
            None
        }
    }
    fn remove_thread(&mut self, thread: &ThreadType) {
        // 移除相应的线程并且确认恰移除一个线程
        let old_len = self.pool.len();
        self.pool.retain(|t| t != thread);
        assert_eq!(self.pool.len() + 1, old_len);
    }
    fn set_priority(&mut self, _thread: ThreadType, _priority: ()) {}
}
