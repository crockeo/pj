use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread;

trait SyncStream {
    type Item;

    /// put adds a new item to the stream.
    fn put(&self, value: Self::Item);

    /// end signifies an end of the stream. Remaining items that were previously inserted with put
    /// will be flushed before notifying consuming threads that there are no more items remaining.
    fn end(&self);

    /// get retrieves a value from the SyncWriteReader. When no value exists, but the stream hasn't
    /// ended (signified by a call to end), get blocks. If the stream has ended *and there are no
    /// remaining values* get returns None instead.
    fn get(&self) -> Option<Self::Item>;
}

struct MutexSyncStreamState<T> {
    is_over: bool,
    elements: Vec<T>,
}

impl<T> MutexSyncStreamState<T> {
    fn new() -> Self {
        Self {
            is_over: false,
            elements: Vec::new(),
        }
    }
}

/// A naive always-locking implementation of a SyncStream.
struct MutexSyncStream<T> {
    state: Mutex<MutexSyncStreamState<T>>,
    state_change: Condvar,
}

impl<T> MutexSyncStream<T> {
    fn new() -> Self {
        Self {
            state: Mutex::new(MutexSyncStreamState::<T>::new()),
            state_change: Condvar::new(),
        }
    }
}

impl<T> SyncStream for MutexSyncStream<T> {
    type Item = T;

    fn put(&self, value: Self::Item) {
        let mut state = self.state.lock().unwrap();
        if state.is_over {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.push(value);
        self.state_change.notify_one();
    }

    fn end(&self) {
        let mut state = self.state.lock().unwrap();
        if state.is_over {
            panic!("attempting to end a stream twice");
        }
        state.is_over = true;
        self.state_change.notify_all();
    }

    fn get(&self) -> Option<Self::Item> {
        let mut state = self.state.lock().unwrap();
        state = self
            .state_change
            .wait_while(state, |state| {
                !(*state).is_over && state.elements.len() == 0
            })
            .unwrap();
        if state.is_over {
            None
        } else {
            state.elements.pop()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;
    use std::thread;

    fn run_sync_stream_test<T: SyncStream<Item = i64> + Send + Sync + 'static>(sync_stream: T) {
        let sync_stream = Arc::new(sync_stream);

        let write_threads = 2;
        let step = 1000 / write_threads;
        let mut write_handles = Vec::new();

        for i in 0..write_threads {
            let min = i * step;
            let max = (i + 1) * step;

            let sync_stream = sync_stream.clone();
            write_handles.push(thread::spawn(move || {
                for j in min..max {
                    sync_stream.as_ref().put(j);
                }
            }));
        }

        let read_threads = 20;
        let mut read_handles = Vec::new();
        for _ in 0..read_threads {
            let sync_stream = sync_stream.clone();
            read_handles.push(thread::spawn(move || {
                let mut read_values = Vec::new();
                while let Some(value) = sync_stream.get() {
                    read_values.push(value);
                }
                read_values
            }));
        }

        for write_handle in write_handles.into_iter() {
            write_handle.join().expect("failed to unwrap write handles");
        }
        sync_stream.as_ref().end();

        let mut seen_values = vec![0; 1000];
        for read_handle in read_handles.into_iter() {
            let read_values = read_handle.join().expect("failed to join read handle");
            for read_value in read_values.into_iter() {
                seen_values[read_value as usize] += 1;
            }
        }

        for seen_value in seen_values.into_iter() {
            assert_eq!(seen_value, 1);
        }
    }

    #[test]
    fn test_mutex_sync_stream() {
        let ss = MutexSyncStream::<i64>::new();
        run_sync_stream_test(ss);
    }
}
