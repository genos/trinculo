use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use trinculo::{
    Interpreter, baseline,
    expr::{self, parse},
    utils::{self, read_prospero, write_image},
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

/// Which tool to use
#[derive(Debug, Clone, ValueEnum)]
enum Tool {
    /// Baseline interpreter of base expressions.
    Baseline,
}

#[derive(Debug, Parser)]
#[command(version, about = "Render the Prospero quote", long_about = None)]
struct Args {
    /// Where to write the output
    #[arg(short, long)]
    output: PathBuf,
    #[arg(short, long, default_value_t = Pixels::Normal, value_enum)]
    pixels: Pixels,
    #[arg(short, long, default_value_t = Tool::Baseline, value_enum)]
    tool: Tool,
}

/// Errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Utils error: {0}")]
    Utils(#[from] utils::Error),
    #[error("Parsing error: {0}")]
    Parse(#[from] expr::ParseError),
    #[error("Baseline interpretation error: {0}")]
    Baseline(#[from] baseline::Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let image_size = u32::from(args.pixels);
    let input = read_prospero()?;
    let program = parse(&input)?;
    match args.tool {
        Tool::Baseline => write_image(
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
