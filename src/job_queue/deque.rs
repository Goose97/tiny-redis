use super::JobQueue;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::{thread, time};

/// This implemention using VecDeque as storage layer
/// To mimic blocking behaviour when dequeue, use loop to keep polling
#[derive(Clone)]
pub struct Queue<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl<T> JobQueue<T> for Queue<T> {
    fn enqueue(&self, item: T) {
        let mut guard = self.queue.lock().unwrap();
        (*guard).push_back(item);
    }

    // Since there's only one consumer, don't bother locking
    fn dequeue(&self) -> T {
        loop {
            match self.queue.lock().unwrap().pop_front() {
                Some(value) => return value,
                // Spin lock
                None => thread::sleep(time::Duration::from_millis(1)),
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
        let thread_1 = thread::spawn(move || {
            queue_1.enqueue(1);
            queue_1.enqueue(2);
        });

        let queue_2 = queue.clone();
        let thread_2 = thread::spawn(move || {
            queue_2.enqueue(3);
            thread::sleep(time::Duration::from_secs(2));
            queue_2.enqueue(4);
        });

        thread_1.join().unwrap();
        thread_2.join().unwrap();

        let mut values = vec![];
        for _i in 1..=4 {
            values.push(queue.dequeue());
        }

        values.sort();
        assert_eq!(values, vec![1, 2, 3, 4]);
    }
}
