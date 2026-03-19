//! Render the Prospero quote
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use trinculo::{
    Interpreter, Translator, baseline, expr, parse, read_prospero, reclaim, reuse, thread_par,
    utils, write_image,
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

/// Which toolset to use
#[derive(Debug, Clone, ValueEnum)]
enum Toolset {
    /// No translation, baseline interpretation
    NopBaseline,
    /// Reuse previously seen expressions, baseline interpretation
    ReuseBaseline,
    /// Reclaiming no longer used expressions
    Reclaim,
    /// Reuse previously seen expressions, reclaim no longer used expressions
    ReuseReclaim,
    /// Thread-based parallel interpretation
    ThreadPar,
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
    /// Which toolset to use
    #[arg(short, long, value_enum)]
    toolset: Toolset,
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
    #[error("Reclaim translation error: {0}")]
    Reclaim(#[from] reclaim::Error),
    #[error("Thread-Parallel interpretation error: {0}")]
    ThreadPar(#[from] thread_par::Error),
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let args = Args::parse();
    let image_size = u32::from(args.pixels);
    let input = read_prospero()?;
    let program = parse(&input)?;
    let image = match args.toolset {
        Toolset::NopBaseline => baseline::Baseline(image_size).interpret(program)?,
        Toolset::ReuseBaseline => {
            baseline::Baseline(image_size).interpret(reuse::Reuse.translate(program)?)?
        }
        Toolset::Reclaim => {
            let r = reclaim::Reclaim(image_size);
            r.interpret(r.translate(program)?)?
        }
        Toolset::ReuseReclaim => {
            let r = reclaim::Reclaim(image_size);
            r.interpret(r.translate(reuse::Reuse.translate(program)?)?)?
        }
        Toolset::ThreadPar => thread_par::ThreadParallel(image_size).interpret(program)?,
    };
    write_image(image_size, image, args.output)?;
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
