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
    delimiter: Option<char>,

    #[arg(short, long)]
    /// set chop boundary the greatest multiple available, limited by terminal width (or `--characters`)
    multiple: Option<usize>,

    #[arg(short, long)]
    /// adjust the chop multiple boundary by a given offset
    offset: Option<usize>,

    #[arg(short, long, default_value = "2.0")]
    /// minimum interval to requery if terminal size has been adjusted; ignored when `--characters` is specified
    update: Option<f32>,
}

struct Limiter {
    config: Config,
    get_termsize: fn() -> Option<termsize::Size>,
}

impl Limiter {
    fn new(config: &Config) -> Self {
        Limiter {
            config: *config,
            get_termsize: termsize::get,
        }
    }

    fn get_limit(&mut self) -> usize {
        if let Some(sz) = self.config.characters {
            return sz;
        }

        let default: usize = match (self.get_termsize)() {
            Some(x) => x.cols as usize,
            None => 80,
        };

        match self.config.multiple {
            Some(0) => default,
            Some(mult) => {
                let offs = self.config.offset.unwrap_or(0);
                ((default - offs) / mult) * mult + offs
            }
            None => default,
        }
    }
}

fn run(
    config: &Config,
    input: &mut impl std::io::BufRead,
    output: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut buffer = String::new();
    let mut limiter = Limiter::new(config);
    loop {
        buffer.clear();
        let nread = input.read_line(&mut buffer)?;

        // in detached stdin state (e.g., daemon), treat as okay
        // TODO: determine if zero-char read should be an error
        if nread == 0 {
            return Ok(());
        }

        let limit = limiter.get_limit();
        let end = match buffer.char_indices().nth(limit) {
            Some(idx_char) => idx_char.0,
            None => buffer.len(),
        };
        let subs = &buffer[..end].trim_end();

        writeln!(output, "{}", subs)?;
    }
}

fn main() {
    let config = Config::parse();

    match run(
        &config,
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    ) {
        Ok(_) => {}
        Err(_) => {
            println!("failure");
        }
    }
}
