//! Render the Prospero quote
use argh::{FromArgValue, FromArgs};
use std::path::PathBuf;
use trinculo::{
    Interpreter, Translator, baseline, combo_par, expr, parse, read_prospero, reclaim,
    reuse, simd_par, thread_par, utils, write_image,
};

/// Pixel size to render.
#[derive(Debug, Clone, FromArgValue)]
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

/// Which translation(s) to use.
#[derive(Debug, Clone, FromArgValue)]
enum Translation {
    Reuse,
}

/// Which interpretation to use.
#[derive(Debug, Clone, FromArgValue)]
enum Interpretation {
    Baseline,
    Reclaim,
    Thread,
    Simd,
    Combo,
}

/// Render the Prospero quote
#[derive(FromArgs)]
struct Args {
    /// where to write the output
    #[argh(option, short = 'o')]
    output: PathBuf,
    /// pixel size to render
    #[argh(option, short = 'p')]
    pixels: Pixels,
    /// which translation(s) to use
    #[argh(option, short = 't')]
    translations: Vec<Translation>,
    /// which interpretation to use
    #[argh(option, short = 'i')]
    interpretation: Interpretation,
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
    let args: Args = argh::from_env();
    let image_size = u16::from(args.pixels);
    let input = read_prospero()?;
    let mut program = parse(&input)?;
    for t in args.translations {
        match t {
            Translation::Reuse => program = reuse::Reuse.translate(program)?,
        }
    }
    let image = match args.interpretation {
        Interpretation::Baseline => baseline::Baseline(image_size).interpret(program)?,
        Interpretation::Reclaim => {
            let r = reclaim::Reclaim(image_size);
            r.interpret(r.translate(program)?)?
        }
        Interpretation::Thread => thread_par::ThreadParallel(image_size).interpret(program)?,
        Interpretation::Simd => simd_par::SimdParallel(image_size).interpret(program)?,
        Interpretation::Combo => combo_par::ComboParallel(image_size).interpret(program)?,
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
