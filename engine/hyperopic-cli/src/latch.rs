use std::cmp::max;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;

/// A CountDownLatch is used to wait for a given number of tasks to be completed 
/// which may be running in multiple threads
pub struct CountDownLatch {
    count: AtomicI64,
    waiters: Mutex<Vec<mpsc::Sender<()>>>,
}

impl CountDownLatch {
    /// Construct a CountDownLatch with the given count
    pub fn new(count: u32) -> Self {
        Self {
            count: AtomicI64::new(count as i64),
            waiters: Mutex::new(vec![]),
        }
    }
    /// Decrement the count by one
    pub fn count_down(&self) {
        if self.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            // We uniquely decremented to 0 so notify everyone waiting
            self.waiters.lock().unwrap().iter().for_each(|tx| tx.send(()).unwrap());
        }
    }

    /// Load the remaining latch count
    pub fn get_current_count(&self, ordering: Ordering) -> usize {
        max(0i64, self.count.load(ordering)) as usize
    }
    
    /// Get a receiver channel which will be notified when the latch count
    /// reaches 0. If the count is already 0 a notification is sent immediately.
    pub fn register_join(&self) -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel();
        if self.get_current_count(Ordering::SeqCst) == 0 {
            tx.send(()).unwrap();
        } else {
            let mut lock = self.waiters.lock().unwrap();
            // The latch may have been released in the time it took to get the lock 
            // so check it again now we have the lock
            if self.get_current_count(Ordering::SeqCst) == 0 {
                tx.send(()).unwrap();
            } else {
                lock.push(tx);
            }
        }
        rx
    }
}