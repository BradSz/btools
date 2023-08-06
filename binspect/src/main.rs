use clap::Parser;

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Endian {
    Big,
    Little,
    Native,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum Format {
    U8,
    U16,
    U32,
    U64, // unsigned
    I8,
    I16,
    I32,
    I64, // signed
    F32,
    F64, // float point
    Hex,
    Oct, // hexdump
    Ascii,
    Utf8,
    Utf16,
    Utf32, // character encoding
}

#[derive(Parser, Debug, Default)]
#[command(author, version, about, long_about = None, propagate_version = true)]
struct Config {
    #[arg()]
    /// Filename to inspect
    file: String,

    /// Load file in interactive
    #[arg(short, long)]
    interactive: bool,

    /// Format of the data to display within the file
    #[arg(short, long, value_enum, default_value = "hex")]
    format: Option<Format>,

    /// Specify endianness of the data
    #[arg(short, long, value_enum, default_value = "native")]
    endian: Option<Endian>,
}

fn main() {
    let config = Config::parse();
    println!("{:?}", &config);
}
