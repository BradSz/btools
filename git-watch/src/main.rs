use anyhow::Result;
use clap::Parser;
use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    time::{Duration, Instant},
};

#[derive(Parser, Default, Debug, Clone)]
#[command(author, version, about, long_about=None, propagate_version=true)]
struct Config {
    /// Command(s) to execute
    #[clap(num_args = 1..)]
    command: Vec<String>,

    #[arg(short = 'a', long, default_value = "30")]
    /// Age of cache to be periodically pruned, in seconds
    age: f32,

    #[arg(short = 'n', long, default_value = "1000")]
    /// Maximum number of elements to retain in cache
    size: usize,

    #[arg(short, long, default_value = "0.2")]
    /// Time allowed for the filesystem to settle before launching command
    settle: f32,

    #[arg(short, long)]
    /// Disable most output
    quiet: bool,

    #[arg(short, long)]
    /// Enable verbose output (overrides --quiet)
    verbose: bool,
}

struct Cache {
    config: Config,
    filenames: HashMap<String, bool>,
    eviction_times: VecDeque<CacheMeta>,
}
struct CacheMeta {
    eviction_time: Instant,
    path: String,
}

impl Cache {
    fn new(config: Config) -> Self {
        Self {
            config,
            filenames: HashMap::new(),
            eviction_times: VecDeque::new(),
        }
    }

    fn is_actionable(&mut self, path: &str) -> bool {
        !self.is_ignored(path)
    }

    fn is_ignored(&mut self, path: &str) -> bool {
        let now = Instant::now();

        // evict cache entries when tracking too many
        while self.eviction_times.len() >= self.config.size {
            if let Some(cache_meta) = self.eviction_times.pop_front() {
                self.filenames.remove(&cache_meta.path);
            }
        }

        // evict cache entries when entries are deemed too old
        loop {
            if let Some(cache_meta) = self.eviction_times.front() {
                if cache_meta.eviction_time < now {
                    self.filenames.remove(&cache_meta.path);
                    let evicted = self.eviction_times.pop_front().unwrap();
                    log::debug!("Stale cache evicted for file \"{}\"", evicted.path);
                    continue; // potentially more to evict
                }
            }
            break; // nothing more to evict
        }

        // use prior cache value
        if let Some(&is_ignored) = self.filenames.get(path) {
            log::debug!(
                "Using cached result {:?} for file {:?}",
                if is_ignored { "ignored" } else { "actionable" },
                path
            );
            return is_ignored;
        }

        // determine if the file is tracked (errors mean not ignored)
        let git_output = std::process::Command::new("git")
            .args(["check-ignore", "--quiet", path])
            .output()
            .expect("failed to execute git");

        let is_ignored = git_output.status.success();

        // cache results
        self.filenames.insert(path.to_string(), is_ignored);
        self.eviction_times.push_back(CacheMeta {
            eviction_time: now + Duration::from_secs_f32(self.config.age),
            path: path.to_string(),
        });

        log::debug!(
            "Determined new result {:?} for file {:?}",
            if is_ignored { "ignored" } else { "actionable" },
            path
        );

        is_ignored
    }
}

fn init_logger(config: &Config) {
    let level = if config.verbose {
        log::LevelFilter::Debug
    } else if config.quiet {
        log::LevelFilter::Error
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::new()
        .format_level(false)
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter(None, level)
        .init();
}

fn main() -> Result<()> {
    let config = Config::parse();
    init_logger(&config);

    log::debug!("{:#?}", config);

    anyhow::ensure!(!config.command.is_empty(), "no command argument provided");

    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("unable to determine git root")
        .stdout;
    let root = String::from_utf8(root).expect("unable to parse root path");
    let root = root.trim();

    log::info!("Running with root: {}", root);

    let mut cache = Cache::new(config.clone());
    let fname = "Cargo.toml";
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));

    let fname = "../target/Cargo.toml";
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));
    println!("Actionable {} : {}", fname, cache.is_actionable(fname));

    // Quick test to execute the command
    let user_command = std::process::Command::new(&config.command[0])
        .args(&config.command[1..])
        .status();

    let status = match user_command {
        Ok(s)=>s,
        Err(_)=>{anyhow::bail!("command not found: {}", &config.command[0])}
    };

    if status.success() {
        log::debug!("Command success: {:?}", config.command);
    } else {
        log::debug!("Command failure: {:?}", config.command);
    }

    Ok(())
}
