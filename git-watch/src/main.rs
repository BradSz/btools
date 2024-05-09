use anyhow::Result;
use clap::Parser;

#[derive(Parser, Default, Debug, Clone)]
#[command(author, version, about, long_about=None, propagate_version=true)]
struct Config {
    #[arg(short='a', long, default_value="30")]
    /// Age of cache to be periodically pruned, in seconds
    max_cache_age: f32,

    #[arg(short='n', long, default_value="1000")]
    /// Maximum number of elements to retain in cache
    max_cache_size: usize,

    #[arg(short, long, default_value = "0.2")]
    /// Time allowed for the filesystem to settle before launching command
    settle: f32,

    #[arg(short, long)]
    /// Command(s) to execute
    command: String,
}

fn main() -> Result<()> {
    let config = Config::parse();

    println!("{:#?}", config);

    Ok(())
}
