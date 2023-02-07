use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use crossbeam::sync::WaitGroup;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use regex::Regex;
use structopt::StructOpt;

// TODO: add the option to ignore certain directories like
// - node_modules
// - venv
// - go (for your $GOPATH)

fn main() -> anyhow::Result<()> {
    let args = Opt::from_args();
    let wait_group = WaitGroup::new();

    let ctx = Arc::new(Context {
	pool: ThreadPoolBuilder::new().build()?,
	max_depth: args.depth,
	sentinel: args.make_sentinel_regex()?,
    });

    for root_dir in args.root_dirs.into_iter() {
        let work_item = Job {
	    ctx: ctx.clone(),
	    wait_group: wait_group.clone(),
            // TODO: resolve symlinks for original directories(?)
            // I'm not sure if this is needed, because read_dir()
            // might just work through symlinks :)
            path: root_dir,
            depth: 0,
        };
        ctx.pool.spawn(move || work_item.job());
    }

    wait_group.wait();
    Ok(())
}

struct Context {
    pool: ThreadPool,
    max_depth: Option<usize>,
    sentinel: Regex,
}

impl Context {
    fn is_match(&self, file_name: &str) -> bool {
	self.sentinel.is_match(file_name)
    }

    fn exceeds_max_depth(&self, depth: usize) -> bool {
	if let Some(max_depth) = self.max_depth {
	    depth >= max_depth
	} else {
	    false
	}
    }
}

struct Job {
    ctx: Arc<Context>,
    wait_group: WaitGroup,
    path: PathBuf,
    depth: usize,
}

impl Job {
    fn child(&self, new_path: PathBuf) -> Self {
        Job {
	    ctx: self.ctx.clone(),
	    wait_group: self.wait_group.clone(),
            path: new_path,
            depth: self.depth + 1,
        }
    }

    fn job(self) {
        match self.job_impl() {
            Err(e) => eprintln!("{:?}", e),
            Ok(_) => {}
        }
	drop(self.wait_group);
    }

    fn job_impl(&self) -> anyhow::Result<()> {
	let should_enqueue = !self.ctx.exceeds_max_depth(self.depth + 1);

        let mut found_paths = Vec::new();
        let mut found_sentinel = false;
        for dir_entry in self.path.read_dir()?.filter_map(Result::ok) {
            let file_name = dir_entry.file_name();
            let file_name = file_name
                .to_str()
                .ok_or_else(|| anyhow!("Cannot convert file_name {:?} to str", file_name))?;

            if self.ctx.is_match(file_name) {
                println!(
                    "{}",
                    self.path
                        .to_str()
                        .ok_or_else(|| anyhow!("Cannot convert path {:?} to str", self.path))?
                );
                found_sentinel = true;
                break;
            }

	    if !should_enqueue {
		continue;
	    }

            // TODO: make this not loop forever when there are recursive symlinks?
            let mut path = dir_entry.path();
            while path.is_symlink() {
                path = fs::read_link(path)?;
            }
            if path.is_dir() {
                found_paths.push(dir_entry.path());
            }
        }

        if !found_sentinel {
            for found_path in found_paths {
                let child = self.child(found_path);
                self.ctx.pool.spawn(move || child.job());
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
