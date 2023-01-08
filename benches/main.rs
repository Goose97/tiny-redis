#[macro_use]
extern crate bencher;
use std::thread::{self, JoinHandle};
use tiny_redis::job_queue::{channel_queue, deque, JobQueue};

use bencher::Bencher;

const N_THREAD: i32 = 8;

/// Benchmark job_queue implementations
/// The benchmark scenarios consist of:
/// 1. Single-thread enqueue (100 items)
/// 2. Multi-thread enqueue (100 items per thread)
/// 3. Single-thread dequeue (1 items)

fn deque_single_thread_enqueue(bench: &mut Bencher) {
    bench.iter(|| {
        let queue = deque::Queue::new();
        for i in 1..=100 {
            queue.enqueue(i);
        }
    })
}

fn deque_multi_thread_enqueue(bench: &mut Bencher) {
    bench.iter(|| {
        let queue = deque::Queue::new();

        for _i in 1..=N_THREAD {
            let clone = queue.clone();
            thread::spawn(move || {
                for i in 1..=100 {
                    clone.enqueue(i);
                }
            });
        }
    })
}

fn deque_single_thread_dequeue(bench: &mut Bencher) {
    let queue = deque::Queue::new();
    for i in 1..=1_000_000 {
        queue.enqueue(i);
    }

    bench.iter(|| {
        queue.dequeue();
    })
}

fn channel_queue_single_thread_enqueue(bench: &mut Bencher) {
    bench.iter(|| {
        let queue = channel_queue::Queue::new();
        for i in 1..=100 {
            queue.enqueue(i);
        }
    })
}

fn channel_queue_multi_thread_enqueue(bench: &mut Bencher) {
    bench.iter(|| {
        let queue = channel_queue::Queue::new();

        let join_handles: Vec<JoinHandle<_>> = (1..=N_THREAD)
            .map(|_| {
                let clone = queue.clone();
                thread::spawn(move || {
                    for i in 1..=100 {
                        clone.enqueue(i);
                    }
                })
            })
            .collect();

        for handle in join_handles {
            handle.join().unwrap();
        }
    })
}

fn channel_queue_multi_thread_dequeue(bench: &mut Bencher) {
    let queue = channel_queue::Queue::new();
    for i in 1..=1_000_000 {
        queue.enqueue(i);
    }

    bench.iter(|| {
        queue.dequeue();
    })
}

benchmark_group!(
    job_queue,
    deque_single_thread_enqueue,
    deque_multi_thread_enqueue,
    deque_single_thread_dequeue,
    channel_queue_single_thread_enqueue,
    channel_queue_multi_thread_enqueue,
    channel_queue_multi_thread_dequeue
);
benchmark_main!(job_queue);
