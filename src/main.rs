extern crate num_cpus;

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::spawn;

struct SearchPool {
    candidates: Mutex<(usize, Vec<PathBuf>)>,
    results: Mutex<Vec<PathBuf>>,
    target: String,

    // work_change signifies a situation in which a worker thread waiting on `get_candidate` should
    // wake up and check its conditions;
    work_change: Condvar,
}

impl SearchPool {
    fn new<S: AsRef<str>, P: AsRef<Path>>(search_target: S, root_path: P) -> SearchPool {
        SearchPool {
            candidates: Mutex::new((0, vec![root_path.as_ref().to_owned()])),
            results: Mutex::new(vec![]),
            target: search_target.as_ref().to_owned(),
            work_change: Condvar::new(),
        }
    }

    fn get_candidate(&self) -> Option<PathBuf> {
        let mut candidates_guard = self.candidates.lock().unwrap();

        candidates_guard.0 += 1;
        while candidates_guard.0 < num_cpus::get() && candidates_guard.1.len() == 0 {
            candidates_guard = self.work_change.wait(candidates_guard).unwrap();
        }

        if candidates_guard.1.len() > 0 {
            candidates_guard.0 -= 1;
            candidates_guard.1.pop()
        } else {
            self.work_change.notify_one();
            None
        }
    }

    fn extend_candidates(&self, paths: Vec<PathBuf>) {
        let mut candidates_guard = self.candidates.lock().unwrap();
        candidates_guard.1.extend(paths);
        self.work_change.notify_all();
    }

    fn put_result(&self, path: PathBuf) {
        self.results.lock().unwrap().push(path);
    }

    fn is_target<P: AsRef<Path>>(&self, path: P) -> io::Result<bool> {
        let file_name = path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid file name"))?;
        Ok(file_name == self.target)
    }

    fn find_sentinel_dirs(self) -> io::Result<Vec<PathBuf>> {
        let self_arc = Arc::new(self);

        let core_count = num_cpus::get();
        let mut children = Vec::with_capacity(core_count);
        for _ in 0..core_count {
            let self_arc = self_arc.clone();
            children.push(spawn(move || {
                // panic if we receive an error, so it bubbles up while joining
                // TODO: find a better way to represent this
                worker(self_arc).unwrap();
            }));
        }

        for child in children.into_iter() {
            child
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "worker thread panicked"))?;
        }

        let results = self_arc.as_ref().results.lock().unwrap().clone();
        Ok(results)
    }
}

fn worker(pool: Arc<SearchPool>) -> io::Result<()> {
    while let Some(path) = pool.as_ref().get_candidate() {
        let sub_paths: Vec<PathBuf> = path
            .read_dir()?
            .flat_map(|dir_entry| io::Result::<_>::Ok(dir_entry?.path()))
            .collect();

        let mut candidate_sub_paths: Vec<PathBuf> = Vec::with_capacity(sub_paths.len());
        let mut found_sentinel = false;
        for sub_path in sub_paths.into_iter() {
            if pool.is_target(&sub_path)? {
                pool.as_ref().put_result(sub_path);
                found_sentinel = true;
                break;
            }

            if sub_path.is_dir() {
                candidate_sub_paths.push(sub_path);
            }
        }

        if !found_sentinel {
            pool.as_ref().extend_candidates(candidate_sub_paths);
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: pj <sentinel file name> [root directory]");
        return Ok(());
    }

    let sentinel_name = args[1].clone();
    let mut root_dir;
    if args.len() >= 3 {
        root_dir = PathBuf::new();
        root_dir.push(args[2].clone());
    } else {
        root_dir = env::current_dir()?;
    }

    let dirs = SearchPool::new(sentinel_name, root_dir).find_sentinel_dirs()?;
    for dir in dirs.into_iter() {
        println!(
            "{}",
            dir.as_path().to_str().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid utf-8 path name"
            ))?
        );
    }

    Ok(())
}
