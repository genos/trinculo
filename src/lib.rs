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
