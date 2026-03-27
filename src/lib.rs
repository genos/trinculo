//! Playing with [Matt Keeter's Prospero Challenge](https://www.mattkeeter.com/projects/prospero/).
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![feature(portable_simd)]

/// Translates input to output, while perhaps dealing with errors.
pub trait Translator {
    type Input;
    type Output;
    type Error;
    /// Run the translation.
    /// # Errors
    /// If something goes wrong during translation.
    fn translate(&self, i: Self::Input) -> Result<Self::Output, Self::Error>;
}

/// Interprets input into a collection of bytes, while perhaps dealing with errors.
pub trait Interpreter {
    type Input;
    type Error;
    /// Run the interpretation.
    /// # Errors
    /// If something goes wrong during interpretation.
    fn interpret(&self, i: Self::Input) -> Result<Vec<u8>, Self::Error>;
}

pub mod baseline;
pub mod combo_par;
pub mod expr;
pub mod reclaim;
pub mod reuse;
pub mod simd_par;
pub mod thread_par;
pub mod unused;
pub mod utils;

pub use expr::parse;
pub use utils::{read_prospero, write_image};

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Log error: {0}")]
    Log(#[from] log::SetLoggerError),
    #[error("Utils error: {0}")]
    Utils(#[from] utils::Error),
    #[error("Parsing error: {0}")]
    Parse(#[from] expr::ParseError),
    #[error("Reuse translation error: {0}")]
    Reuse(#[from] reuse::Error),
    #[error("Unused translation error: {0}")]
    Unused(#[from] unused::Error),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Program;
    use chaos_theory::check;
    #[test]
    fn equivalent_interpretations() {
        let n = 8;
        let input = read_prospero();
        assert!(input.is_ok());
        let prog = parse(&input.unwrap());
        assert!(prog.is_ok());
        let prog = prog.unwrap();
        check(|src| {
            fn interpret(name: &str, n: u16, prog: Program) -> Result<Vec<u8>, Error> {
                Ok(match name {
                    "baseline" => baseline::Baseline(n).interpret(prog)?,
                    "combo" => combo_par::ComboParallel(n).interpret(prog)?,
                    "reclaim" => {
                        let r = reclaim::Reclaim(n);
                        r.interpret(r.translate(prog)?)?
                    }
                    "simd" => simd_par::SimdParallel(n).interpret(prog)?,
                    "thread" => thread_par::ThreadParallel(n).interpret(prog)?,
                    _ => unreachable!(),
                })
            }
            let first = src.select(
                "1st",
                &["baseline", "combo", "reclaim", "simd", "thread"],
                |_, name, _| interpret(name, n, prog.clone()),
            );
            assert!(first.is_ok());
            let second = src.select(
                "2nd",
                &["baseline", "combo", "reclaim", "simd", "thread"],
                |_, name, _| interpret(name, n, prog.clone()),
            );
            assert!(second.is_ok());
            assert_eq!(first.unwrap(), second.unwrap());
        });
    }
}

#[cfg(test)]
macro_rules! snapshot_test {
    ($name:expr) => {
        $crate::snapshot_test!($name, 1024);
    };
    ($name:expr, $n:literal) => {
        use super::*;
        use crate::{
            expr::parse,
            utils::{read_prospero, to_image, to_png},
        };
        #[test]
        fn snapshot() {
            let n = $n;
            let input = read_prospero();
            assert!(input.is_ok());
            let program = parse(&input.unwrap());
            assert!(program.is_ok());
            let output = $name(n).interpret(program.unwrap());
            assert!(output.is_ok());
            let image = to_image(n, output.unwrap());
            assert!(image.is_ok());
            let png = to_png(&image.unwrap());
            assert!(png.is_ok());
            insta::assert_binary_snapshot!("snapshot.png", png.unwrap());
        }
    };
    ($name:expr, $n:literal, @translate) => {
        use super::*;
        use crate::{
            expr::parse,
            utils::{read_prospero, to_image, to_png},
        };
        #[test]
        fn snapshot() {
            let n = $n;
            let input = read_prospero();
            assert!(input.is_ok());
            let program = parse(&input.unwrap());
            assert!(program.is_ok());
            let f = $name(n);
            let translated = f.translate(program.unwrap());
            assert!(translated.is_ok());
            let output = f.interpret(translated.unwrap());
            assert!(output.is_ok());
            let image = to_image(n, output.unwrap());
            assert!(image.is_ok());
            let png = to_png(&image.unwrap());
            assert!(png.is_ok());
            insta::assert_binary_snapshot!("snapshot.png", png.unwrap());
        }
    };
}
#[cfg(test)]
pub(crate) use snapshot_test;
