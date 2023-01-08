use super::JobQueue;
use std::sync::mpsc;

pub struct Queue<T> {
    sender: mpsc::Sender<T>,
    receiver: Option<mpsc::Receiver<T>>,
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: None,
        }
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Some(receiver),
        }
    }
}

impl<T> JobQueue<T> for Queue<T> {
    fn enqueue(&self, item: T) -> () {
        self.sender.send(item).unwrap();
    }

    fn dequeue(&self) -> T {
        if let Some(receiver) = &self.receiver {
            receiver.recv().unwrap()
        } else {
            panic!("Cloned channel_queue::Queue can't not dequeue items")
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
