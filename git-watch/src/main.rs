use anyhow::Result;
use clap::Parser;
use std::io::Write;

#[derive(Parser, Default, Debug, Clone)]
#[command(author, version, about, long_about=None, propagate_version=true)]
struct Config {
    /// Command(s) to execute
    command: String,

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

    log::info!("{:#?}", config);

    Ok(())
}
