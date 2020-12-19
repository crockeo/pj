use std::sync::{Condvar, Mutex};

pub trait SyncStream {
    type Item;

    fn with_writers(writer_count: usize) -> Self;

    fn put(&self, value: Self::Item);
    fn end(&self) -> bool;
    fn get(&self) -> Option<Self::Item>;
}

struct MutexSyncStreamState<T> {
    writer_count: usize,
    ended_count: usize,
    elements: Vec<T>,
}

impl<T> MutexSyncStreamState<T> {
    fn is_over(&self) -> bool {
        self.ended_count >= self.writer_count
    }
}

impl<T> MutexSyncStreamState<T> {
    fn with_writers(writer_count: usize) -> Self {
        Self {
            writer_count: writer_count,
            ended_count: 0,
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

    fn with_writers(writer_count: usize) -> Self {
        Self {
            state: Mutex::new(MutexSyncStreamState::with_writers(writer_count)),
            state_change: Condvar::new(),
        }
    }

    fn put(&self, value: Self::Item) {
        let mut state = self.state.lock().unwrap();
        if state.is_over() {
            panic!("attempted to write to stream after it was closed");
        }

        state.elements.push(value);
        self.state_change.notify_one();
    }

    fn end(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.is_over() {
            panic!("attempting to end a stream twice");
        }

        state.ended_count += 1;
        loop {
            if state.is_over() {
                self.state_change.notify_all();
                return true;
            } else if state.elements.len() > 0 {
                state.ended_count -= 1;
                return false;
            } else {
                state = self.state_change.wait(state).unwrap();
            }
        }
    }

    fn get(&self) -> Option<Self::Item> {
        let mut state = self.state.lock().unwrap();
        state.elements.pop()
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
                sync_stream.end();
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
        let ss = MutexSyncStream::<i64>::with_writers(2);
        run_sync_stream_test(ss);
    }
}
