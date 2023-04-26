use clap::Parser;

#[derive(Parser, Default, Debug, Clone, Copy)]
#[command(author, version, about, long_about = None, propagate_version = true)]
struct Config {
    #[arg(short, long)]
    /// Wrap lines at boundary instead of truncating
    wrap: Option<bool>,

    #[arg(short, long)]
    /// Chop after given number of columns instead of screen width
    columns: Option<usize>,

    #[arg(short, long)]
    /// Chop after the last of a given delimiter in a line, limited by terminal width (or `--columns`)
    delimiter: Option<char>,

    #[arg(short, long)]
    /// Set chop boundary the greatest multiple available, limited by terminal width (or `--columns`)
    multiple: Option<usize>,

    #[arg(short, long)]
    /// Adjust the chop multiple boundary by a given offset
    offset: Option<usize>,

    #[arg(short, long, default_value = "2.0")]
    /// Minimum interval to requery if terminal size has been adjusted; ignored when `--columns` is specified
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
        let default = {
            match self.config.columns {
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

fn get_end(s: &str, limit: usize, delim: Option<char>) -> usize {
    use std::cmp::min;

    let s_len = s.len();

    if s_len < limit {
        return s_len; // already fits in allowed space
    }

    let mut trial = min(limit, s_len); // default if no delimiter found
    let mut col: usize = 0;
    for (c_idx, c_val) in s.char_indices() {
        col += unicode_width::UnicodeWidthChar::width(c_val).unwrap_or(1);

        if col > limit {
            break; // break before updating trial, so wide characters are pushed over
        }

        if c_val == delim.unwrap_or(c_val) {
            trial = c_idx;
        }
    }

    min(s_len, trial + 1)
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
        while !s.is_empty() {
            let limit = limiter.get_limit();
            let end = get_end(s, limit, config.delimiter);
            let subs = &s[..end];
            if let Err(e) = writeln!(output, "{}", subs) {
                match e.kind() {
                    std::io::ErrorKind::BrokenPipe => {
                        return Ok(());
                    }
                    _ => {
                        return Err(e);
                    }
                }
            }

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
    /// Verify that lines are chopped after terminal bounds,
    /// assuming terminal is 10 columns wide.
    fn test_default() {
        let config = Config::default();
        let mut limiter = Limiter {
            config,
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

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    /// Verify that lines are wrapped (and continued) at terminal bounds,
    /// assuming terminal is 30 columns wide.
    fn test_wrap() {
        let config = Config {
            wrap: Some(true),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
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
            "[10char-D]",                     // line 1 (wrap)
            "[10char-E][10char-F]",           // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    /// Verify that supplying a `columns` option overrides terminal bounds
    /// assuming columns is set larger than terminal size.
    fn test_wrap_chars_when_larger() {
        let config = Config {
            wrap: Some(true),
            columns: Some(20),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
            get_termsize: get_termsize_10,
        };

        let input: String = format!(
            "{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D]", // line 1
            "[10char-E][10char-F]",                     // line 2
        );

        let exp: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B]", // line 1
            "[10char-C][10char-D]", // line 1 (wrap)
            "[10char-E][10char-F]", // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    /// Verify that supplying a `columns` option overrides terminal bounds
    /// assuming columns is set smaller than terminal size.
    fn test_wrap_chars_when_smaller() {
        let config = Config {
            wrap: Some(true),
            columns: Some(20),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
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
            "[10char-C][10char-D]", // line 1 (wrap)
            "[10char-E][10char-F]", // line 2
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    /// Verify that supplying a `multiple` flag will wrap at the greatest
    /// multiple that is strictly less than the specified column limit.
    fn test_wrap_chars_multiple() {
        let config = Config {
            wrap: Some(true),
            columns: Some(55),
            multiple: Some(20),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
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
            "[10char-E][10char-F]",                     // line 1 (wrap)
            "[10char-G][10char-H][10char-I]",           // line 2
            "[10char-J][10char-K][10char-L][10char-M]", // line 3
            "[10char-N]",                               // line 3 (wrap)
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    fn test_wrap_chars_multiple_offset() {
        let config = Config {
            wrap: Some(true),
            columns: Some(55),
            multiple: Some(20),
            offset: Some(10),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D][10char-E][10char-F]", // line 1
            "[10char-G][10char-H][10char-I]",                               // line 2
            "[10char-J][10char-K][10char-L][10char-M][10char-N]",           // line 3
        );

        let exp: String = format!(
            "{}\n{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D][10char-E]", // line 1
            "[10char-F]",                                         // line 1 (wrap)
            "[10char-G][10char-H][10char-I]",                     // line 2
            "[10char-J][10char-K][10char-L][10char-M][10char-N]", // line 3
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    #[test]
    fn test_default_chars_multiple() {
        let config = Config {
            wrap: Some(false),
            columns: Some(55),
            multiple: Some(20),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
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

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string);
    }

    #[test]
    fn test_wrap_delimiter() {
        let config = Config {
            wrap: Some(true),
            delimiter: Some('-'),
            ..Default::default()
        };
        let mut limiter = Limiter {
            config,
            get_termsize: get_termsize_30,
        };

        let input: String = format!(
            "{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-C][10char-D][10char-E][10char-F]", // line 1
            "[10char-G][10char-H][10char-I]",                               // line 2
            "[10char-J][10char-K][10char-L][10char-M][10char-N]",           // line 3
        );

        let exp: String = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            "[10char-A][10char-B][10char-",   // line 1
            "C][10char-D][10char-E][10char-", // line 1 (wrap)
            "F]",                             // line 1 (wrap)
            "[10char-G][10char-H][10char-",   // line 2
            "I]",                             // line 2 (wrap)
            "[10char-J][10char-K][10char-",   // line 3
            "L][10char-M][10char-N]",         // line 3 (wrap)
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    // #[test]
    // fn test_non_ascii_unicode_narrow() {
    //     todo!();
    //     // "🌈";
    // }

    #[test]
    fn test_non_ascii_unicode_wide() {
        let config = Config::default();
        let mut limiter = Limiter {
            config,
            get_termsize: get_termsize_30,
        };

        let c = '🌈';
        assert_eq!(2, unicode_width::UnicodeWidthChar::width(c).unwrap());

        let input: String = format!(
            "{}\n{}\n{}\n{}\n",
            "[10char-🌈][10char-B][10char-C]",    // line 1 (wide)
            "[10char-🌈][10char-E][10char-🌈]", // line 2 (wide)
            "[10-a̐éö̲-🌈][10-a̐éö̲-E][10-a̐éö̲-🌈]", // line 3 (wide and graphemes)
            "[10char-🌈]",                        // line 4 (wide)
        );

        let exp: String = format!(
            "{}\n{}\n{}\n{}\n",
            "[10char-🌈][10char-B][10char-C", // line 1 (chopped two columns)
            "[10char-🌈][10char-E][10char-",  // line 2 (chopped three columns)
            "[10-a̐éö̲-🌈][10-a̐éö̲-E][10-a̐éö̲-", // line 2 (chopped with graphemes)
            "[10char-🌈]",                    // line 3
        );

        let mut output: Vec<u8> = Vec::new();
        run(&config, &mut limiter, &mut input.as_bytes(), &mut output).unwrap();

        let output_string = String::from_utf8(output).unwrap();
        assert_eq!(exp, output_string, "\n{}\n", output_string);
    }

    // #[test]
    // fn test_non_unicode_bytes() {
    //     todo!();
    // }
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
