use std::sync::{Condvar, Mutex};

pub trait SyncStream {
    type Item;

    fn with_threads(thread_count: usize) -> Self;
    fn put(&self, value: Self::Item);
    fn get(&self) -> Option<Self::Item>;

    // default extend assumes no special circumstances about knowing the length of the values ahead
    // of time. can be overriden to optimize for specific implementations
    fn extend(&self, values: Vec<Self::Item>) {
        for value in values.into_iter() {
            self.put(value);
        }
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
}

impl<T> MutexSyncStreamState<T> {
    fn with_threads(thread_count: usize) -> Self {
        Self {
            thread_count: thread_count,
            waiting_count: 0,
            elements: Vec::new(),
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

    fn put(&self, value: Self::Item) {
        let mut state = self.state.lock().unwrap();
        if state.is_stalled() {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.push(value);
        self.state_change.notify_one();
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

    fn extend(&self, values: Vec<Self::Item>) {
        let mut state = self.state.lock().unwrap();
        if state.is_stalled() {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.extend(values);
        self.state_change.notify_all();
    }
}

/// A lock-light implementation of a SyncStream. Attempts to use reference swapping to reduce the
/// frequency of locking.
struct SwapSyncStream<T> {
    placeholder_to_remove: T,
}

impl<T> SyncStream for SwapSyncStream<T> {
    type Item = T;

    fn with_threads(thread_count: usize) -> Self { todo!() }

    fn put(&self, value: Self::Item) { todo!() }
    fn get(&self) -> Option<Self::Item> { todo!() }
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
}
