use clap::Parser;

#[derive(Parser, Debug, Clone, Copy)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Config {
    #[arg(short, long)]
    /// Wrap lines at boundary instead of truncating
    wrap: Option<bool>,

    #[arg(short, long)]
    /// Chop after given number of characters instead of screen width
    characters: Option<usize>,

    #[arg(short, long)]
    /// Chop after the last of a given delimiter in a line, limited by terminal width (or `--characters`)
    delimiter: Option<char>,

    #[arg(short, long)]
    /// Set chop boundary the greatest multiple available, limited by terminal width (or `--characters`)
    multiple: Option<usize>,

    #[arg(short, long)]
    /// Adjust the chop multiple boundary by a given offset
    offset: Option<usize>,

    #[arg(short, long, default_value = "2.0")]
    /// Minimum interval to requery if terminal size has been adjusted; ignored when `--characters` is specified
    update: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wrap: Default::default(),
            characters: Default::default(),
            delimiter: Default::default(),
            multiple: Default::default(),
            offset: Default::default(),
            update: Default::default(),
        }
    }
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
        let default = {
            match self.config.characters {
                Some(sz) => sz,
                None => match (self.get_termsize)() {
                    Some(x) => x.cols as usize,
                    None => 80,
                },
            }
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
    limiter: &mut Limiter,
    input: &mut impl std::io::BufRead,
    output: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let mut buffer = String::new();
    loop {
        buffer.clear();
        let nread = input.read_line(&mut buffer)?;

        // in detached stdin state (e.g., daemon), treat as okay
        // TODO: determine if zero-char read should be an error
        if nread == 0 {
            return Ok(());
        }

        let mut s = buffer.as_str().trim_end();
        while s.len() != 0 {
            let limit = limiter.get_limit();
            let end = match s.char_indices().nth(limit) {
                Some(idx_char) => idx_char.0,
                None => s.len(),
            };
            let subs = &s[..end].trim_end();
            writeln!(output, "{}", subs)?;

            if config.wrap.unwrap_or(false) {
                s = &s[end..];
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_termsize_10() -> Option<termsize::Size> {
        Some(termsize::Size { rows: 0, cols: 10 })
    }

    fn get_termsize_30() -> Option<termsize::Size> {
        Some(termsize::Size { rows: 0, cols: 30 })
    }

    #[test]
    fn test_truncate_default() {
        let config = Config::default();
        let mut limiter = Limiter {
            config: config,
            get_termsize: get_termsize_10,
        };

        let input: String = format!(
            "{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-E][10char-F]",                     // line 2
        );
        let exp: String = format!(
            "{}\n{}\n",
            "[10char-A]", // line 1
            "[10char-E]", // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        assert_eq!(exp.as_bytes(), output, "\n{}\n", input);
    }

    #[test]
    fn test_wrap() {
        let mut config = Config::default();
        config.wrap = Some(true);
        let mut limiter = Limiter {
            config: config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-E][10char-F]",                     // line 2
        );

        let exp: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C]", // line 1
            "[10char-D]",                     // line 1
            "[10char-E][10char-F]",           // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        assert_eq!(exp.as_bytes(), output, "\n{}\n", input);
    }

    #[test]
    fn test_wrap_chars() {
        let mut config = Config::default();
        config.wrap = Some(true);
        config.characters = Some(20);
        let mut limiter = Limiter {
            config: config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-E][10char-F]",                     // line 2
        );

        let exp: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B]", // line 1
            "[10char-C][10char-D]", // line 1
            "[10char-E][10char-F]", // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        assert_eq!(exp.as_bytes(), output, "\n{}\n", input);
    }

    #[test]
    fn test_wrap_chars_multiple() {
        let mut config = Config::default();
        config.wrap = Some(true);
        config.characters = Some(55);
        config.multiple = Some(20);
        let mut limiter = Limiter {
            config: config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D][10char-E][10char-F]", // line 1
            "[10char-G][10char-H][10char-I]",                               // line 2
            "[10char-J][10char-K][10char-L][10char-M][10char-N]",           // line 3
        );

        let exp: String = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-E][10char-F]",                     // line 1
            "[10char-G][10char-H][10char-I]",           // line 2
            "[10char-J][10char-K][10char-L][10char-M]", // line 3
            "[10char-N]",                               // line 3
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        assert_eq!(exp.as_bytes(), output, "\n{}\n", input);
    }

    #[test]
    fn test_default_chars_multiple() {
        let mut config = Config::default();
        config.wrap = Some(false);
        config.characters = Some(55);
        config.multiple = Some(20);
        let mut limiter = Limiter {
            config: config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D][10char-E][10char-F]", // line 1
            "[10char-G][10char-H][10char-I]",                               // line 2
            "[10char-J][10char-K][10char-L][10char-M][10char-N]",           // line 3
        );

        let exp: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-G][10char-H][10char-I]",           // line 2
            "[10char-J][10char-K][10char-L][10char-M]", // line 3
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        assert_eq!(exp.as_bytes(), output, "\n{}\n", input);
    }
}

fn main() {
    let config = Config::parse();

    match run(
        &config,
        &mut Limiter::new(&config),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    ) {
        Ok(_) => {}
        Err(_) => {
            println!("failure");
        }
    }
}
