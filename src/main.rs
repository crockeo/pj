pub mod sync_reader;
pub mod worker;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use regex::Regex;
use structopt::StructOpt;

use crate::sync_reader::SyncStream;

fn main() -> io::Result<()> {
    let args = Opt::from_args();

    // Regex doesn't have a is_full_match function.
    // We ensure the regex starts with `^` and ends with `$`
    // so that any match is a full match.
    let mut sentinel_pattern_str = args.sentinel_pattern;
    if !sentinel_pattern_str.starts_with("^") {
        sentinel_pattern_str = format!("^{sentinel_pattern_str}");
    }
    if !sentinel_pattern_str.ends_with("$") {
        sentinel_pattern_str = format!("{sentinel_pattern_str}$");
    }

    let sentinel_pattern =
        Regex::new(&sentinel_pattern_str).expect("Failed to create Regex from provided sentinel");

    let cpus = num_cpus::get();
    let work_target = Arc::new(worker::WorkTarget {
        sentinel_pattern,
        sync_stream: sync_reader::SwapSyncStream::with_threads(cpus),
        max_depth: args.depth,
    });

    let mut root_dirs = args.root_dirs;
    if root_dirs.len() == 0 {
        root_dirs.push(env::current_dir()?);
    }
    work_target
        .sync_stream
        .extend(root_dirs.into_iter().map(|path| worker::WorkItem {
            path: fs::canonicalize(path).expect("Could not canonicalize path"),
            depth: 0,
        }));

    let mut workers = Vec::with_capacity(cpus);
    for _ in 0..cpus {
        let work_target = work_target.clone();
        workers.push(thread::spawn(move || worker::finder_worker(work_target)));
    }

    for worker in workers.into_iter() {
        worker.join().expect("failed to join worker");
    }

    Ok(())
}

#[derive(StructOpt)]
#[structopt(name = "pj", about = "A fast sentinel file finder.")]
struct Opt {
    sentinel_pattern: String,

    root_dirs: Vec<PathBuf>,

    #[structopt(short, long)]
    depth: Option<usize>,
}
