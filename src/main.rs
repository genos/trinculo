//! Render the Prospero quote
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use trinculo::{
    Interpreter, Translator, baseline, combo_par, expr, parse, read_prospero, reclaim, reuse,
    simd_par, thread_par, utils, write_image,
};

/// Pixel size to render.
#[derive(Debug, Clone, ValueEnum)]
enum Pixels {
    /// 256 pixels
    Small,
    /// 1024 pixels
    Normal,
    /// 4096 pixels
    Big,
}

impl From<Pixels> for u16 {
    fn from(p: Pixels) -> Self {
        match p {
            Pixels::Small => 256,
            Pixels::Normal => 1024,
            Pixels::Big => 4096,
        }
    }
}

/// Which toolset to use.
#[derive(Debug, Clone, ValueEnum)]
enum Toolset {
    /// No translation, baseline interpretation.
    NopBaseline,
    /// Reuse previously seen expressions, baseline interpretation.
    ReuseBaseline,
    /// Reclaiming no longer used expressions.
    NopReclaim,
    /// Reuse previously seen expressions, reclaim no longer used expressions.
    ReuseReclaim,
    /// Thread-based parallel interpretation.
    NopThreadPar,
    /// SIMD-based parallel interpretation.
    NopSimdPar,
    /// Combination of SIMD- and thread-based parallel interpretation.
    NopComboPar,
    /// Reuse previously seen expressions, Thread-based parallel interpretation.
    ReuseThreadPar,
    /// Reuse previously seen expressions, SIMD-based parallel interpretation.
    ReuseSimdPar,
    /// Reuse previously seen expressions, combination of SIMD- and thread-based parallel
    /// interpretation.
    ReuseComboPar,
}

#[derive(Debug, Parser)]
#[command(version, about = "Render the Prospero quote", long_about = None)]
struct Args {
    /// Where to write the output.
    #[arg(short, long)]
    output: PathBuf,
    /// Pixel size to render.
    #[arg(short, long, default_value_t = Pixels::Normal, value_enum)]
    pixels: Pixels,
    /// Which toolset to use.
    #[arg(short, long, value_enum)]
    toolset: Toolset,
}

/// Errors
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Log error: {0}")]
    Log(#[from] log::SetLoggerError),
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
    #[error("SIMD-Parallel interpretation error: {0}")]
    SimdPar(#[from] simd_par::Error),
    #[error("SIMD-Parallel interpretation error: {0}")]
    ComboPar(#[from] combo_par::Error),
}

fn main() -> Result<(), Error> {
    simple_logger::init_with_env()?;
    let args = Args::parse();
    let image_size = u16::from(args.pixels);
    let input = read_prospero()?;
    let program = parse(&input)?;
    let image = match args.toolset {
        Toolset::NopBaseline => baseline::Baseline(image_size).interpret(program)?,
        Toolset::ReuseBaseline => {
            baseline::Baseline(image_size).interpret(reuse::Reuse.translate(program)?)?
        }
        Toolset::NopReclaim => {
            let r = reclaim::Reclaim(image_size);
            r.interpret(r.translate(program)?)?
        }
        Toolset::ReuseReclaim => {
            let r = reclaim::Reclaim(image_size);
            r.interpret(r.translate(reuse::Reuse.translate(program)?)?)?
        }
        Toolset::NopThreadPar => thread_par::ThreadParallel(image_size).interpret(program)?,
        Toolset::NopSimdPar => simd_par::SimdParallel(image_size).interpret(program)?,
        Toolset::NopComboPar => combo_par::ComboParallel(image_size).interpret(program)?,
        Toolset::ReuseThreadPar => {
            thread_par::ThreadParallel(image_size).interpret(reuse::Reuse.translate(program)?)?
        }
        Toolset::ReuseSimdPar => {
            simd_par::SimdParallel(image_size).interpret(reuse::Reuse.translate(program)?)?
        }
        Toolset::ReuseComboPar => {
            combo_par::ComboParallel(image_size).interpret(reuse::Reuse.translate(program)?)?
        }
    };
    write_image(image_size, image, args.output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pixel_size() {
        assert_eq!(u16::from(Pixels::Small), 256);
        assert_eq!(u16::from(Pixels::Normal), 1024);
        assert_eq!(u16::from(Pixels::Big), 4096);
    }
}
