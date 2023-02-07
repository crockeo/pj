use std::path::PathBuf;
use std::sync::Arc;

use crossbeam::sync::WaitGroup;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    let args = Opt::from_args();
    let pool = Arc::new(ThreadPoolBuilder::new().build()?);
    let wait_group = WaitGroup::new();
    let sentinel = Arc::new(args.make_sentinel_regex()?);

    for root_dir in args.root_dirs.into_iter() {
        let work_item = WorkItem {
            pool: pool.clone(),
            wait_group: wait_group.clone(),
            max_depth: args.depth,
            sentinel: sentinel.clone(),
            path: root_dir,
            depth: 0,
        };
        pool.spawn(move || work_item.job());
    }

    wait_group.wait();
    Ok(())
}

struct WorkItem {
    pool: Arc<ThreadPool>,
    wait_group: WaitGroup,
    max_depth: Option<usize>,
    sentinel: Arc<Regex>,
    path: PathBuf,
    depth: usize,
}

impl WorkItem {
    fn child(&self, new_path: PathBuf) -> Self {
        WorkItem {
            pool: self.pool.clone(),
            wait_group: self.wait_group.clone(),
            max_depth: self.max_depth,
            sentinel: self.sentinel.clone(),
            path: new_path,
            depth: self.depth + 1,
        }
    }

    fn job(self) {
        match self.job_impl() {
            Err(e) => eprintln!("{:?}", e),
            Ok(_) => {}
        }
    }

    fn job_impl(self) -> anyhow::Result<()> {
        let mut found_paths = Vec::new();
        let mut found_sentinel = false;
        for dir_entry in self.path.read_dir()?.filter_map(Result::ok) {
            let file_name = dir_entry.file_name();
            let file_name = file_name.to_str()?;

            if self.sentinel.is_match(file_name) {
                println!("{}", self.path.to_str()?);
                found_sentinel = true;
                break;
            }

            if dir_entry.metadata()?.is_dir() {
                found_paths.push(dir_entry.path());
            }
        }

        if let Some(max_depth) = self.max_depth {
            if self.depth >= max_depth {
                return Ok(());
            }
        }

        if !found_sentinel {
            for found_path in found_paths {
                let child = self.child(found_path);
                self.pool.spawn(move || child.job());
            }
        }

        Ok(())
    }
}

#[derive(StructOpt)]
#[structopt(name = "pj", about = "A fast sentinel file finder.")]
struct Opt {
    sentinel_pattern: String,

    root_dirs: Vec<PathBuf>,

    #[structopt(short, long)]
    depth: Option<usize>,
}

impl Opt {
    fn make_sentinel_regex(&self) -> anyhow::Result<Regex> {
        // Regex doesn't have a is_full_match function.
        // We ensure the regex starts with `^` and ends with `$`
        // so that any match is a full match.
        let prefix = if self.sentinel_pattern.starts_with("^") {
            ""
        } else {
            "^"
        };
        let suffix = if self.sentinel_pattern.ends_with("$") {
            ""
        } else {
            "$"
        };
        let sentinel_str = format!("{prefix}{}{suffix}", self.sentinel_pattern);
        Ok(Regex::new(&sentinel_str)?)
    }
}
