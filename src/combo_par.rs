//! A combination of SIMD- and thread-based parallel interpretation of [`crate::expr::Expr`]s.
use crate::{Interpreter, expr::Program, simd_par};
use rayon::prelude::*;
use std::{simd::prelude::*, time::Instant};

/// Combination of SIMD- and thread-based parallel interpreter.
///
/// Given an image size (in pixels per side), the [`Interpreter`] instance will interpret the
/// [`crate::expr::Expr`]s listed in a [`Program`] in parallel.
pub struct ComboParallel(pub u16);

const N: usize = 64;

/// Errors that can arise when interpreting a [`Program`] with a [`ComboParallel`] interpreter.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

/// Reuses [`crate::simd_par`]'s internal `exec()` function for any given pixel.
impl Interpreter for ComboParallel {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, p: Program) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        let image_size = usize::from(self.0);
        let mut out = vec![0u8; image_size * image_size];
        out.par_chunks_exact_mut(N)
            .enumerate()
            .for_each(|(i, c)| simd_par::exec(i, c, &p.exprs, &Simd::splat(image_size)));
        let elapsed = start.elapsed();
        log::info!("Combination Parallel Interpreter: time = {elapsed:?}");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        expr::parse,
        utils::{read_prospero, to_image, to_png},
    };
    #[test]
    fn simd_par_16() {
        let input = read_prospero();
        assert!(input.is_ok());
        let input = input.unwrap();
        let program = parse(&input);
        assert!(program.is_ok());
        let output = ComboParallel(16).interpret(program.unwrap());
        assert!(output.is_ok());
        let image = to_image(16, output.unwrap());
        assert!(image.is_ok());
        let png = to_png(&image.unwrap());
        assert!(png.is_ok());
        insta::assert_binary_snapshot!("combo_par_16.png", png.unwrap());
    }
}
