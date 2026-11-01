//! 确定性比例份额调度：总选择最小 pass，每次增加 BIG / priority。
use super::Scheduler;
use alloc::vec::Vec;
const BIG: u128 = 1 << 40;
struct Entry<T> { thread: T, pass: u128, priority: usize }
pub struct StrideScheduler<T> { pool: Vec<Entry<T>> }
impl<T> Default for StrideScheduler<T> {
    fn default() -> Self { Self { pool: Vec::new() } }
}
impl<T: Clone + Eq> Scheduler<T> for StrideScheduler<T> {
    type Priority = usize;
    fn add_thread(&mut self, thread: T) {
        assert!(!self.pool.iter().any(|item| item.thread == thread));
        // 新到达/唤醒线程从当前虚拟时间开始，避免长时间睡眠后突发霸占 CPU。
        let pass = self.pool.iter().map(|item| item.pass).min().unwrap_or(0);
        self.pool.push(Entry { thread, pass, priority: 1 });
    }
    fn get_next(&mut self) -> Option<T> {
        let index = self.pool.iter().enumerate().min_by_key(|(_, item)| item.pass)?.0;
        if self.pool.iter().any(|item| item.pass > u128::MAX - BIG) {
            let floor = self.pool[index].pass;
            for item in &mut self.pool { item.pass -= floor; }
        }
        let item = &mut self.pool[index];
        item.pass += BIG / item.priority as u128;
        Some(item.thread.clone())
    }
    fn remove_thread(&mut self, thread: &T) {
        let previous = self.pool.len();
        self.pool.retain(|item| item.thread != *thread);
        assert_eq!(previous, self.pool.len() + 1);
    }
    fn set_priority(&mut self, thread: T, priority: usize) {
        assert!(priority > 0 && priority <= 1_000_000, "invalid priority");
        let item = self.pool.iter_mut().find(|item| item.thread == thread).expect("unknown thread");
        item.priority = priority;
    }
}
/// 与内核共用的算法测试，不依赖宿主机时钟。
pub fn stride_self_test() {
    let mut scheduler = StrideScheduler::<usize>::default();
    assert_eq!(scheduler.get_next(), None);
    scheduler.add_thread(1);
    scheduler.add_thread(2);
    scheduler.set_priority(1, 9);
    let mut counts = [0usize; 2];
    for _ in 0..10000 { counts[scheduler.get_next().unwrap() - 1] += 1; }
    assert!(counts[0] >= 8999 && counts[0] <= 9001);
    assert!(counts[1] >= 999 && counts[1] <= 1001);
    scheduler.remove_thread(&2);
    for _ in 0..100 { assert_eq!(scheduler.get_next(), Some(1)); }
    scheduler.add_thread(2);
    let mut count = 0;
    for _ in 0..100 { if scheduler.get_next() == Some(2) { count += 1; } }
    assert!(count >= 9 && count <= 11);
    scheduler.set_priority(1, 1);
    let mut count = 0;
    for _ in 0..1000 { if scheduler.get_next() == Some(1) { count += 1; } }
    assert!(count >= 498 && count <= 502);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proportional_shares_and_wakeup() { stride_self_test(); }
    #[test]
    #[should_panic(expected = "invalid priority")]
    fn reject_zero() {
        let mut scheduler = StrideScheduler::default();
        scheduler.add_thread(1);
        scheduler.set_priority(1, 0);
    }
    #[test]
    fn normalize_before_overflow() {
        let mut scheduler = StrideScheduler::default();
        scheduler.add_thread(1);
        scheduler.add_thread(2);
        for item in &mut scheduler.pool { item.pass = u128::MAX - 1; }
        assert!(scheduler.get_next().is_some());
        assert!(scheduler.get_next().is_some());
    }
}
