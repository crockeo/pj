use std::path::PathBuf;
use std::sync::Arc;

use regex::Regex;

use crate::sync_reader::SyncStream;

// TODO: hide these internal fields and provide a constructor to map from Opt to WorkTarget (with a
// particular SyncStream implemenetation)
pub struct WorkItem {
    pub path: PathBuf,
    pub depth: usize,
}

pub struct WorkTarget<T: SyncStream<Item = WorkItem>> {
    pub sentinel_pattern: Regex,
    pub sync_stream: T,
    pub max_depth: Option<usize>,
}

impl<T: SyncStream<Item = WorkItem>> WorkTarget<T> {
    fn exceeds_depth(&self, depth: usize) -> bool {
        match self.max_depth {
            None => false,
            // >, rather than >=, is intended here.
            // the 0th directory is the root dir,
            // so we want to seach down `max_depth` more levels
            Some(max_depth) => depth > max_depth,
        }
    }
}

pub fn finder_worker<T: SyncStream<Item = WorkItem>>(
    target: Arc<WorkTarget<T>>,
) {
    while let Some(work_item) = target.sync_stream.get() {
        let mut candidate_subpaths = Vec::new();
        let mut found_sentinel = false;

        let dir_entries = match work_item.path.read_dir() {
            Err(_) => continue,
            Ok(x) => x,
        };
        for dir_entry in dir_entries.filter_map(|dir_entry| dir_entry.ok()) {
            let raw_file_name = dir_entry.file_name();
            let file_name = raw_file_name
                .to_str()
                .expect("failed to convert OsStr -> str");
            if target.sentinel_pattern.is_match(file_name) {
                println!("{}", work_item.path.to_str().unwrap());
                found_sentinel = true;
                break;
            }

            if dir_entry.metadata().map(|m| m.is_dir()).unwrap_or(false) && !target.exceeds_depth(work_item.depth + 1) {
                candidate_subpaths.push(WorkItem {
                    path: dir_entry.path(),
                    depth: work_item.depth + 1,
                });
            }
        }

        if !found_sentinel {
            target.sync_stream.extend(candidate_subpaths);
        }
    }
}
