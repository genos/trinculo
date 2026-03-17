use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use trinculo::{
    Interpreter, Translator, baseline, expr, parse, read_prospero, reuse, utils, write_image,
};

/// Pixel size to render
#[derive(Debug, Clone, ValueEnum)]
enum Pixels {
    /// 256 pixels
    Small,
    /// 1024 pixels
    Normal,
    /// 4096 pixels
    Big,
}

impl From<Pixels> for u32 {
    fn from(p: Pixels) -> Self {
        match p {
            Pixels::Small => 256,
            Pixels::Normal => 1024,
            Pixels::Big => 4096,
        }
    }
}

/// Which Translator to use
#[derive(Debug, Clone, ValueEnum)]
enum Translation {
    /// No translation
    Nop,
    /// Reuse previously seen expressions
    Reuse,
}

/// Which Interpreter to use
#[derive(Debug, Clone, ValueEnum)]
enum Interpretation {
    /// Baseline interpreter of expressions.
    Baseline,
}

#[derive(Debug, Parser)]
#[command(version, about = "Render the Prospero quote", long_about = None)]
struct Args {
    /// Where to write the output
    #[arg(short, long)]
    output: PathBuf,
    /// Pixel size to render
    #[arg(short, long, default_value_t = Pixels::Normal, value_enum)]
    pixels: Pixels,
    /// Which translator(s) to use
    #[arg(short, long, value_enum)]
    translators: Vec<Translation>,
    /// Which interpreter to use
    #[arg(short, long, default_value_t = Interpretation::Baseline, value_enum)]
    interpreter: Interpretation,
}

/// Errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Utils error: {0}")]
    Utils(#[from] utils::Error),
    #[error("Parsing error: {0}")]
    Parse(#[from] expr::ParseError),
    #[error("Reuse translation error: {0}")]
    Reuse(#[from] reuse::Error),
    #[error("Baseline interpretation error: {0}")]
    Baseline(#[from] baseline::Error),
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let args = Args::parse();
    let image_size = u32::from(args.pixels);
    let input = read_prospero()?;
    let mut program = parse(&input)?;
    for t in args.translators {
        match t {
            Translation::Nop => (),
            Translation::Reuse => program = reuse::Reuse.translate(program)?,
        }
    }
    match args.interpreter {
        Interpretation::Baseline => write_image(
            image_size,
            baseline::Baseline(image_size).interpret(program)?,
            args.output,
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixel_size() {
        assert_eq!(u32::from(Pixels::Small), 256);
        assert_eq!(u32::from(Pixels::Normal), 1024);
        assert_eq!(u32::from(Pixels::Big), 4096);
    }
}
