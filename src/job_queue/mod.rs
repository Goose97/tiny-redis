pub mod deque;
pub mod channel_queue;
pub mod disruptor;

// Multiple producers / single consumer job queue
pub trait JobQueue<T> {
  fn enqueue(&self, item: T) -> ();

  // This will block until there's new job to handle
  // hence it will always return a result
  fn dequeue(&self) -> T;
}
