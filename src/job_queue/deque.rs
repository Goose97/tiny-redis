use super::JobQueue;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

/// This implemention using VecDeque as storage layer
/// To mimic blocking behaviour when dequeue, use loop to keep polling
#[derive(Clone)]
pub struct Queue<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
    cvar: Arc<Condvar>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            cvar: Arc::new(Condvar::new()),
        }
    }
}

impl<T> JobQueue<T> for Queue<T> {
    fn enqueue(&self, item: T) {
        let start = Instant::now();
        let mut guard = self.queue.lock().unwrap();
        let duration = start.elapsed();
        log::debug!("job_queue acquire mutex took: {duration:?}");
        (*guard).push_back(item);
        self.cvar.notify_one();
    }

    fn dequeue(&self) -> T {
        let mut guard = self.queue.lock().unwrap();

        loop {
            match guard.pop_front() {
                Some(value) => return value,
                None => guard = self.cvar.wait(guard).unwrap(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::JobQueue;
    use super::Queue;
    use std::{thread, time};

    #[test]
    fn single_thread() {
        let queue = Queue::new();

        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);

        assert_eq!(queue.dequeue(), 1);
        assert_eq!(queue.dequeue(), 2);
        assert_eq!(queue.dequeue(), 3);
    }

    #[test]
    fn multi_thread() {
        let queue = Queue::new();

        let queue_1 = queue.clone();
        thread::spawn(move || {
            queue_1.enqueue(1);
            queue_1.enqueue(2);
        });

        let queue_2 = queue.clone();
        thread::spawn(move || {
            queue_2.enqueue(3);
            thread::sleep(time::Duration::from_secs(2));
            queue_2.enqueue(4);
        });

        let mut values = vec![];
        for _i in 1..=4 {
            values.push(queue.dequeue());
        }

        values.sort();
        assert_eq!(values, vec![1, 2, 3, 4]);
    }
}
