use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

trait SyncReader {
    type Item;

    fn get(&self) -> Option<Self::Item> where Self::Item: Clone;
}

pub struct VecReader<T> {
    items: Vec<T>,
    index: AtomicUsize,
}

impl<T> VecReader<T> {
    pub fn new(items: Vec<T>) -> VecReader<T> {
        VecReader {
            items: items,
            index: AtomicUsize::new(0),
        }
    }
}

impl<T: Clone> SyncReader for VecReader<T> {
    type Item = T;

    fn get(&self) -> Option<T> {
        let item = self.items.get(self.index.fetch_add(1, Ordering::Relaxed));
        // TODO: replace this clone with some sort of move out of vec
        item.map(T::clone)
    }
}

// TODO: create struct that allows for high-perf writing and reading :)

#[cfg(test)]
mod tests {
    use super::*;

    fn run_test_sync_reader_test<T>(reader: T)
    where
        T: SyncReader<Item = u16> + Send + Sync + 'static {
        let reader = Arc::new(reader);

        let thread_count = 16;
        let mut children = Vec::new();
        for _ in 0..thread_count {
            let reader = reader.clone();
            children.push(thread::spawn(move || {
                let mut read_values = Vec::new();
                while let Some(value) = reader.get() {
                    read_values.push(value);
                }
                read_values
            }));
        }

        let mut seen_values = vec![0; 1000];
        for child in children.into_iter() {
            for returned_value in child.join().unwrap().into_iter() {
                seen_values[returned_value as usize] += 1;
            }
        }

        for seen_value in seen_values.into_iter() {
            assert_eq!(seen_value, 1);
        }
    }

    #[test]
    fn test_sync_reader() {
        // simple test that sets up a SyncReader, gets a bunch of threads to read for it, and then
        // ensures no two threads read the same value
        let values: Vec<u16> = (0..1000).collect::<Vec<u16>>();
        let reader = VecReader::new(values);

        run_test_sync_reader_test(reader)
    }
}
