use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

pub trait SyncStream {
    type Item;

    fn with_threads(thread_count: usize) -> Self;
    fn get(&self) -> Option<Self::Item>;
    fn put(&self, value: Self::Item);

    // default extend assumes no special circumstances about knowing the length of the values ahead
    // of time. can be overriden to optimize for specific implementations
    fn extend(&self, values: Vec<Self::Item>) {
        for value in values.into_iter() {
            self.put(value);
        }
    }
}

/// A naive always-locking implementation of a SyncStream.
pub struct MutexSyncStream<T> {
    state: Mutex<MutexSyncStreamState<T>>,
    state_change: Condvar,
}

impl<T> SyncStream for MutexSyncStream<T> {
    type Item = T;

    fn with_threads(thread_count: usize) -> Self {
        Self {
            state: Mutex::new(MutexSyncStreamState::with_threads(thread_count)),
            state_change: Condvar::new(),
        }
    }

    fn get(&self) -> Option<Self::Item> {
        let mut state = self.state.lock().unwrap();
        state.waiting_count += 1;
        loop {
            if state.elements.len() > 0 {
                state.waiting_count -= 1;
                return state.elements.pop();
            } else if state.is_stalled() {
                self.state_change.notify_all();
                return None;
            }
            state = self.state_change.wait(state).unwrap();
        }
    }

    fn put(&self, value: Self::Item) {
        let mut state = self.state.lock().unwrap();
        if state.is_stalled() {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.push(value);
        self.state_change.notify_one();
    }

    fn extend(&self, values: Vec<Self::Item>) {
        let mut state = self.state.lock().unwrap();
        if state.is_stalled() {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.extend(values);
        self.state_change.notify_all();
    }
}

struct MutexSyncStreamState<T> {
    thread_count: usize,
    waiting_count: usize,
    elements: Vec<T>,
}

impl<T> MutexSyncStreamState<T> {
    fn is_stalled(&self) -> bool {
        if self.waiting_count > self.thread_count {
            panic!("waiting count > thread count, should be impossible");
        }
        self.waiting_count >= self.thread_count
    }

    fn with_threads(thread_count: usize) -> Self {
        Self {
            thread_count: thread_count,
            waiting_count: 0,
            elements: Vec::new(),
        }
    }
}

/// An implementation of a SyncStream that separates the writer and reader locks, as well as
/// provides a method to swap between them.
struct SwapSyncStream<T> {
    read_state: Mutex<MutexSyncStreamState<T>>,
    write_state: Mutex<Vec<T>>,
    swap_evt: Condvar,
}

impl<T> SyncStream for SwapSyncStream<T> {
    type Item = T;

    fn with_threads(thread_count: usize) -> Self {
        Self {
            read_state: Mutex::new(MutexSyncStreamState::with_threads(thread_count)),
            write_state: Mutex::new(Vec::new()),
            swap_evt: Condvar::new(),
        }
    }

    fn get(&self) -> Option<Self::Item> {
        let mut read_state = self.read_state.lock().unwrap();
        read_state.waiting_count += 1;
        loop {
            if read_state.elements.len() > 0 {
                read_state.waiting_count -= 1;
                return read_state.elements.pop();
            } else if read_state.is_stalled() {
                let mut write_state = self.write_state.lock().unwrap();
                if write_state.len() == 0 {
                    self.swap_evt.notify_all();
                    return None;
                }

                drain_extend(&mut read_state.elements, &mut write_state);
                self.swap_evt.notify_all();
                continue;
            }
            read_state = self.swap_evt.wait(read_state).unwrap();
        }
    }

    fn put(&self, value: Self::Item) {
        let mut write_state = self.write_state.lock().unwrap();
        write_state.push(value);
    }

    fn extend(&self, values: Vec<Self::Item>) {
        let mut write_state = self.write_state.lock().unwrap();
        write_state.extend(values);
    }
}

fn drain_extend<T>(target: &mut Vec<T>, source: &mut Vec<T>) {
    let new_len = target.len() + source.len();
    // TODO: think about making this a larger bound--like 2x the existing capacity?
    if new_len > target.capacity() {
        target.reserve(new_len - target.capacity());
    }

    for item in source.drain(0..) {
        target.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;
    use std::thread;

    fn run_sync_stream_test<T: SyncStream<Item = i64> + Send + Sync + 'static, F : Fn(usize) -> T>(make_sync_stream: F, thread_count: i64) {
        let sync_stream = Arc::new(make_sync_stream(thread_count as usize));

        let step = 1000 / thread_count;
        let mut children = Vec::new();
        for i in 0..thread_count {
            let min = i * step;
            let max = (i + 1) * step;
            let sync_stream = sync_stream.clone();

            children.push(thread::spawn(move || {
                let mut results = Vec::new();
                for i in min..max {
                    sync_stream.put(i);
                }

                while let Some(value) = sync_stream.get() {
                    results.push(value);
                }
                results
            }));
        }

        let mut seen_counts = vec![0; 1000];
        for child in children {
            let results = child.join().expect("failed to join child");
            for result in results.into_iter() {
                seen_counts[result as usize] += 1;
            }
        }

        for (i, seen_count) in seen_counts.into_iter().enumerate() {
            assert_eq!(seen_count, 1, "{} seen {} time(s)", i, seen_count);
        }
    }

    #[test]
    fn test_mutex_sync_stream() {
        // we run this test 100x so that we have a higher chance of eliciting race conditions
        for _ in 0..100 {
            run_sync_stream_test(MutexSyncStream::<i64>::with_threads, 10);
        }
    }

    #[test]
    fn test_swap_sync_stream() {
        for _ in 0..100 {
            run_sync_stream_test(SwapSyncStream::<i64>::with_threads, 10);
        }
    }
}
