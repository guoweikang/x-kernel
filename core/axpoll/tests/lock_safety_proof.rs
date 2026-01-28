/// This test demonstrates the lock safety improvement in the wake() method.
/// 
/// The key issue was that calling wake() would drop the Inner struct while holding
/// the lock, and Inner::drop() would call wake() on all wakers, which could try to
/// re-acquire the lock and cause deadlock or unexpected behavior.
///
/// Our fix extracts all wakers from the lock first, releases the lock, and then
/// calls wake() on each waker outside the lock.

#[test]
fn test_wake_safety_detailed() {
    use std::{
        sync::{Arc, atomic::{AtomicUsize, Ordering}},
        task::{Wake, Waker},
    };
    use axpoll::PollSet;

    struct ReentrantWaker {
        counter: AtomicUsize,
        poll_set: Option<Arc<PollSet>>,
    }

    impl ReentrantWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                counter: AtomicUsize::new(0),
                poll_set: None,
            })
        }

        fn with_poll_set(poll_set: Arc<PollSet>) -> Arc<Self> {
            Arc::new(Self {
                counter: AtomicUsize::new(0),
                poll_set: Some(poll_set),
            })
        }

        fn count(&self) -> usize {
            self.counter.load(Ordering::SeqCst)
        }
    }

    impl Wake for ReentrantWaker {
        fn wake(self: Arc<Self>) {
            self.counter.fetch_add(1, Ordering::SeqCst);
            
            // This simulates a waker that might try to interact with the PollSet
            // In the old implementation, this could cause deadlock
            if let Some(ref ps) = self.poll_set {
                // Try to register again (simulating re-registration during wake)
                // This should not deadlock because wake() releases the lock first
                let new_waker = Waker::from(ReentrantWaker::new());
                ps.register(&new_waker);
            }
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Test 1: Simple wake without re-entry
    {
        let ps = Arc::new(PollSet::new());
        let waker_obj = ReentrantWaker::new();
        let waker = Waker::from(waker_obj.clone());
        
        ps.register(&waker);
        ps.as_ref().wake();
        
        assert_eq!(waker_obj.count(), 1, "Waker should be called once");
    }

    // Test 2: Wake with potential re-entry (the critical test)
    {
        let ps = Arc::new(PollSet::new());
        let waker_obj = ReentrantWaker::with_poll_set(ps.clone());
        let waker = Waker::from(waker_obj.clone());
        
        ps.register(&waker);
        
        // This should NOT deadlock because:
        // 1. wake() extracts all wakers while holding the lock
        // 2. wake() releases the lock
        // 3. wake() calls waker.wake() outside the lock
        // 4. waker.wake() can safely call ps.register() because lock is free
        ps.as_ref().wake();
        
        assert_eq!(waker_obj.count(), 1, "Waker should be called once");
    }

    // Test 3: Multiple wakers with potential re-entry
    {
        let ps = Arc::new(PollSet::new());
        let mut wakers = Vec::new();
        
        for _ in 0..10 {
            let waker_obj = ReentrantWaker::with_poll_set(ps.clone());
            wakers.push(waker_obj.clone());
            let waker = Waker::from(waker_obj);
            ps.register(&waker);
        }
        
        // Wake all - should not deadlock
        let count = ps.as_ref().wake();
        assert_eq!(count, 10, "Should wake 10 wakers");
        
        for (i, waker) in wakers.iter().enumerate() {
            assert_eq!(waker.count(), 1, "Waker {} should be called once", i);
        }
    }
}
