use super::JobQueue;
use std::cell::SyncUnsafeCell;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/**
 * Disruptor uses a ring buffer to mimic a queue. Ring buffer is a vec, except
 * it wraps when reach the end to form a circular form.
 * It maintains 2 pointer:
 * - tail_cursor: enqueue operation will set into this cursor and advance it
 * - head_cursor: dequeue operation will get from this cursor and advance it
 * Think of these pointers as they're "chasing" each other. As we consume items from
 * the queue, the head_cursor is "chasing" the tail_pointer. As we enqueue items into
 * the queue, the tail_cursor is "chasing" the head_cursor.
 * Which means:
 * - the queue is empty if the head_cursor catchs up with the tail_cursor
 * - the queue is full if the tail_cursor catchs up with the head_cursor
 *
 * We deliberately choose the SIZE as exponential of 2. It makes calculate the index from sequence
 * easier by using bitwise. For instance, a ring buffer of size 4 will wrap when it reaches sequence number 4.
 * The slot is the remainder of sequence number divided by the size, in this case 4 % 4 = 0.
 * This calculation is trivial if the size is a exponential of 2. In the above example, the slot is
 * the last two bits of the sequence number
*/

pub struct Queue<T, const SIZE: usize> {
    head_cursor: Arc<AtomicUsize>,
    tail_cursor: Arc<AtomicUsize>,
    next_slot_cursor: Arc<AtomicUsize>,
    ring: Arc<SyncUnsafeCell<[Option<T>; SIZE]>>,
    size: usize,
    exponential: i8,
}

impl<T: Copy, const SIZE: usize> Queue<T, SIZE> {
    pub fn new() -> Self {
        Self {
            // The last item we dequeue (0 means we haven't dequeue any items)
            head_cursor: Arc::new(AtomicUsize::new(0)),

            // The last item we enqueue (0 means we haven't enqueue any items)
            tail_cursor: Arc::new(AtomicUsize::new(0)),

            // The next sequence we will enqueue
            next_slot_cursor: Arc::new(AtomicUsize::new(1)),

            ring: Arc::new(SyncUnsafeCell::new([None; SIZE])),
            size: SIZE,
            exponential: (SIZE as f64).log2() as i8,
        }
    }

    // We have to make sure we don't overflow the ring by over-claim the sequence
    fn claim_sequence(&self) -> usize {
        loop {
            // Spin loop
            while self.is_full() {}

            let current = self.next_slot_cursor.load(Ordering::Acquire);
            match self.next_slot_cursor.compare_exchange(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(value) => return value,
                Err(_) => continue,
            }
        }
    }

    fn slot_from_sequence(&self, sequence: usize) -> usize {
        ((1 << self.exponential) - 1) & sequence
    }

    fn is_full(&self) -> bool {
        let next_slot = self.next_slot_cursor.load(Ordering::Acquire);
        let head = self.head_cursor.load(Ordering::Acquire);
        next_slot - head - 1 == self.size
    }

    fn is_empty(&self) -> bool {
        let tail = self.tail_cursor.load(Ordering::Acquire);
        let head = self.head_cursor.load(Ordering::Acquire);
        tail == head
    }

    /// Commit the new_tail_cursor. If there are slots between current cursor
    /// and the new cursor, we have to block. For example, our cursor is now 2
    /// and we want to commit to 4. We have to wait till 3 commit to continue our
    /// operation, so we must block
    fn commit(&self, new_tail_cursor: usize) {
        while self
            .tail_cursor
            .compare_exchange(
                new_tail_cursor - 1,
                new_tail_cursor,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {}
    }
}

impl<T: Copy, const SIZE: usize> Clone for Queue<T, SIZE> {
    fn clone(&self) -> Self {
        Self {
            head_cursor: self.head_cursor.clone(),
            tail_cursor: self.tail_cursor.clone(),
            next_slot_cursor: self.next_slot_cursor.clone(),
            ring: self.ring.clone(),
            size: self.size,
            exponential: self.exponential,
        }
    }
}

impl<T: Copy + Debug, const SIZE: usize> JobQueue<T> for Queue<T, SIZE> {
    fn enqueue(&self, item: T) -> () {
        // Claim phase: we only increment next_slot_cursor in this phase
        let next_sequence = self.claim_sequence();
        let next_slot = self.slot_from_sequence(next_sequence);

        // Commit phase: write item to the slot and increment tail_cursor
        // Only after this phase, our item is visible to the dequeue operation
        unsafe {
            let first_item_ptr = (*self.ring.get()).as_mut_ptr();
            let target_ptr = first_item_ptr.add(next_slot);
            *target_ptr = Some(item);
            self.commit(next_sequence);
        }
    }

    fn dequeue(&self) -> T {
        // Spin loop
        while self.is_empty() {}

        // There is only consumer thread which
        self.head_cursor.fetch_add(1, Ordering::Release);
        let consume_sequence = self.head_cursor.load(Ordering::Acquire);
        let consume_slot = self.slot_from_sequence(consume_sequence);

        unsafe {
            let first_item_ptr = (*self.ring.get()).as_mut_ptr();
            let target_ptr = first_item_ptr.add(consume_slot);
            (*target_ptr).take().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::JobQueue;
    use super::Queue;
    use std::thread;

    #[test]
    fn single_thread() {
        let queue = Queue::<_, 4>::new();

        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);
        queue.enqueue(4);

        assert_eq!(queue.dequeue(), 1);
        assert_eq!(queue.dequeue(), 2);
        assert_eq!(queue.dequeue(), 3);
        assert_eq!(queue.dequeue(), 4);
    }

    #[test]
    fn multi_thread() {
        let queue = Queue::<_, 8>::new();

        let queue_1 = queue.clone();
        thread::spawn(move || {
            for i in 1..=100 {
                queue_1.enqueue(i);
            }
        });

        let queue_2 = queue.clone();
        thread::spawn(move || {
            for i in 101..=200 {
                queue_2.enqueue(i);
            }
        });

        let mut values = vec![];
        for _i in 1..=200 {
            values.push(queue.dequeue());
        }

        values.sort();
        assert_eq!(values, (1..=200).collect::<Vec<i32>>());
    }
}
