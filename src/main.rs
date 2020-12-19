extern crate num_cpus;
extern crate shellexpand;

pub mod sync_reader;

use std::env;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crate::sync_reader::SyncStream;

fn finder_worker<T: sync_reader::SyncStream<Item = PathBuf>>(
    target: Arc<String>,
    sync_stream: Arc<T>,
) -> io::Result<Vec<PathBuf>> {
    let mut found_paths = Vec::new();
    while let Some(path_buf) = sync_stream.get() {
        let mut candidate_subpaths: Vec<PathBuf> = Vec::new();
        let mut found_sentinel = false;

        for sub_path in path_buf.read_dir()? {
            let sub_path = sub_path?.path();

            let file_name = sub_path
                .as_path()
                .file_name()
                .expect("failed to get file name")
                .to_str()
                .expect("failed to convert OsStr->str");
            if file_name == target.as_ref() {
                found_paths.push(path_buf);
                found_sentinel = true;
                break;
            }

            if sub_path.is_dir() {
                candidate_subpaths.push(sub_path);
            }
        }

        if !found_sentinel {
            sync_stream.extend(candidate_subpaths);
        }
    }

    Ok(found_paths)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: pj <sentinel file name> [root directory]");
        return Ok(());
    }

    let core_count = num_cpus::get();
    let sync_stream = Arc::new(sync_reader::SwapSyncStream::with_threads(core_count));

    let sentinel_name = Arc::new(args[1].clone());
    if args.len() == 2 {
        sync_stream.put(env::current_dir()?);
    } else {
        for root_dir_str in args[2..].into_iter() {
            let root_dir_str = shellexpand::tilde(root_dir_str);

            let mut root_dir = PathBuf::new();
            root_dir.push(root_dir_str.clone().as_ref());

            sync_stream.put(root_dir);
        }
    }

    let mut workers = Vec::with_capacity(core_count);
    for _ in 0..core_count {
        let sentinel_name = sentinel_name.clone();
        let sync_stream = sync_stream.clone();
        workers.push(thread::spawn(move || {
            finder_worker(sentinel_name, sync_stream)
        }));
    }

    for path in workers.into_iter().flat_map(|w| {
        w.join()
            .expect("failed to join thread")
            .expect("thread failed to execute")
    }) {
        println!("{}", path.to_str().expect("invalid path"));
    }

    Ok(())
}
