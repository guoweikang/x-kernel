use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Wake, Waker},
};

use axpoll::PollSet;

#[cfg(feature = "alloc")]
use axpoll::PollSetGroup;

struct Counter(AtomicUsize);

impl Counter {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicUsize::new(0)))
    }

    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    fn add(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.add();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.add();
    }
}

#[test]
fn test_no_auto_wake_on_drop() {
    let mut ps = PollSet::new_no_auto_wake();
    let counter = Counter::new();
    for _ in 0..10 {
        let w = Waker::from(counter.clone());
        let cx = Context::from_waker(&w);
        ps.register(cx.waker());
    }
    // Disable auto wake on drop
    ps.set_auto_wake_on_drop(false);
    drop(ps);
    // Should not have woken any tasks
    assert_eq!(counter.count(), 0);
}

#[test]
fn test_set_auto_wake_on_drop() {
    let mut ps = PollSet::new();
    let counter = Counter::new();
    for _ in 0..5 {
        let w = Waker::from(counter.clone());
        let cx = Context::from_waker(&w);
        ps.register(cx.waker());
    }
    // Enable auto wake (should be default)
    ps.set_auto_wake_on_drop(true);
    drop(ps);
    // Should have woken all tasks
    assert_eq!(counter.count(), 5);
}

#[test]
fn test_no_deadlock_on_wake() {
    // This test ensures that wake() doesn't deadlock when waker.wake()
    // tries to register again or access the PollSet.
    let ps = Arc::new(PollSet::new());
    let counter = Counter::new();
    
    // Register some wakers
    for _ in 0..10 {
        let w = Waker::from(counter.clone());
        ps.register(&w);
    }
    
    // This should not deadlock
    let woke = ps.as_ref().wake();
    assert_eq!(woke, 10);
    assert_eq!(counter.count(), 10);
}

#[cfg(feature = "stats")]
#[test]
fn test_stats() {
    let ps = PollSet::new();
    let counter = Counter::new();
    
    // Initial stats
    let stats = ps.stats();
    assert_eq!(stats.register_count, 0);
    assert_eq!(stats.wake_count, 0);
    assert_eq!(stats.current_count, 0);
    
    // Register some wakers
    for _ in 0..5 {
        let w = Waker::from(counter.clone());
        ps.register(&w);
    }
    
    let stats = ps.stats();
    assert_eq!(stats.register_count, 5);
    assert_eq!(stats.wake_count, 0);
    assert_eq!(stats.current_count, 5);
    
    // Wake them
    ps.wake();
    
    let stats = ps.stats();
    assert_eq!(stats.register_count, 5);
    assert_eq!(stats.wake_count, 1);
    assert_eq!(stats.current_count, 0);
    
    // Register more
    for _ in 0..3 {
        let w = Waker::from(counter.clone());
        ps.register(&w);
    }
    
    let stats = ps.stats();
    assert_eq!(stats.register_count, 8);
    assert_eq!(stats.wake_count, 1);
    assert_eq!(stats.current_count, 3);
}

#[cfg(feature = "alloc")]
#[test]
fn test_poll_set_group() {
    let mut group = PollSetGroup::new();
    let counter = Counter::new();
    
    // Create and add multiple PollSets
    for _ in 0..3 {
        let ps = PollSet::new();
        for _ in 0..5 {
            let w = Waker::from(counter.clone());
            ps.register(&w);
        }
        group.add(ps);
    }
    
    // Wake all
    let total = group.wake_all();
    assert_eq!(total, 15); // 3 PollSets * 5 wakers each
    assert_eq!(counter.count(), 15);
}

#[cfg(feature = "alloc")]
#[test]
fn test_poll_set_group_register_all() {
    let mut group = PollSetGroup::new();
    
    // Add 3 empty PollSets
    for _ in 0..3 {
        group.add(PollSet::new());
    }
    
    let counter = Counter::new();
    let w = Waker::from(counter.clone());
    
    // Register with all sets
    group.register_all(&w);
    
    // Wake all should wake 3 times (once per PollSet)
    let total = group.wake_all();
    assert_eq!(total, 3);
    assert_eq!(counter.count(), 3);
}

#[cfg(feature = "alloc")]
#[test]
fn test_poll_set_group_default() {
    let group = PollSetGroup::default();
    assert_eq!(group.wake_all(), 0);
}
