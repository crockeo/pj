extern crate num_cpus;
extern crate regex;
extern crate shellexpand;
extern crate structopt;

pub mod sync_reader;
pub mod worker;

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use regex::Regex;
use structopt::StructOpt;

use crate::sync_reader::SyncStream;

fn main() -> io::Result<()> {
    let args = Opt::from_args();

    let cpus = num_cpus::get();
    let work_target = Arc::new(worker::WorkTarget {
        sentinel_pattern: args.sentinel_pattern,
        sync_stream: sync_reader::SwapSyncStream::with_threads(cpus),
        max_depth: args.depth,
    });

    let mut root_dirs = args.root_dirs;
    if root_dirs.len() == 0 {
        root_dirs.push(env::current_dir()?);
    }
    work_target.sync_stream.extend(
        root_dirs
            .into_iter()
            .map(|path| worker::WorkItem { path: expand_tilde(path), depth: 0 }),
    );

    let mut workers = Vec::with_capacity(cpus);
    for _ in 0..cpus {
        let work_target = work_target.clone();
        workers.push(thread::spawn(move || worker::finder_worker(work_target)));
    }

    for worker in workers.into_iter() {
        worker
            .join()
            .expect("failed to join worker");
    }

    Ok(())
}

fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    // TODO: smooth this out a little bit to be more rustic...whatever that means
    let mut buf = PathBuf::new();
    buf.push(shellexpand::tilde(path.as_ref().to_str().unwrap()).into_owned());
    buf
}

#[derive(StructOpt)]
#[structopt(name = "pj", about = "A fast sentinel file finder.")]
struct Opt {
    #[structopt(parse(try_from_str = Regex::new))]
    sentinel_pattern: Regex,

    root_dirs: Vec<PathBuf>,

    #[structopt(short, long)]
    depth: Option<usize>,
}
