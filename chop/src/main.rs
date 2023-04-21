use clap::Parser;

#[derive(Parser, Debug, Clone, Copy)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]

struct Config {
    #[arg(short, long)]
    /// wrap lines at boundary instead of truncating
    wrap: bool,

    #[arg(short, long)]
    /// chop after given number of characters instead of screen width
    characters: Option<usize>,

    #[arg(short, long)]
    /// chop after the last of a given delimiter in a line, limited by terminal width (or `--characters`)
    delimiter: Option<usize>,

    #[arg(short, long)]
    /// set chop boundary the greatest multiple available, limited by terminal width (or `--characters`)
    multiple: Option<usize>,

    #[arg(short = 'o', long)]
    /// adjust the chop multiple boundary by a given offset
    multiple_offset: Option<usize>,

    #[arg(short, long, default_value = "0.5")]
    /// minimum interval to requery if terminal size has been adjusted; ignored when `--characters` is specified
    refresh: Option<f32>,
}

struct Limiter {
    config: Config,
}

impl Limiter {
    fn new(config: &Config) -> Self {
        Limiter { config: *config }
    }

    fn get_limit(&mut self) -> usize {
        let default: usize = match termsize::get() {
            Some(x) => x.cols as usize,
            None => 80,
        };
        match self.config.characters {
            Some(sz) => sz,
            None => default,
        }
    }
}

fn run(config: &Config) -> std::io::Result<()> {
    let mut buffer = String::new();
    let mut limiter = Limiter::new(config);
    loop {
        buffer.clear();
        let nread = std::io::stdin().read_line(&mut buffer)?;
        if nread == 0 {
            // in detached stdin state (e.g., daemon), treat as okay
            // TODO: determine if zero-char read should be an error
            return Ok(());
        }

        let limit = limiter.get_limit();
        let end = match buffer.char_indices().nth(limit) {
            Some(idx_char) => idx_char.0,
            None => buffer.len(),
        };
        let subs = &buffer[..end].trim_end();

        // std::io::stdout().write(&buffer)?;
        println!("{}", subs);
    }
}

fn main() {
    let config = Config::parse();

    match run(&config) {
        Ok(_) => {
            println!("success");
        }
        Err(_) => {
            println!("failure");
        }
    }
}
