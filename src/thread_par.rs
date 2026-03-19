//! Thread-based parallel interpretation of [`Expr`]s.
use crate::{Interpreter, baseline, expr::Program};
use rayon::prelude::*;
use std::time::Instant;

/// Thread-based parallel interpreter; given an image size (in pixels per side), the
/// [`Interpreter`] instance will interpret the [`Expr`]s listed in a [`Program`] in parallel.
pub struct ThreadParallel(pub u32);

/// Errors that can arise when interpreting a [`Program`] with a [`Parallel`] interpreter.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("u32 size is too large to fit into a usize: {0}")]
    TooBigSize(u32),
    #[error("u32 size is too large to fit half of it into an f32: {0}")]
    TooBigF32(u32),
}

/// Reuses [`crate::baseline`]'s internal `run()` function for any given pixel.
impl Interpreter for ThreadParallel {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, p: Program) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        let image_size = usize::try_from(self.0).map_err(|_| Error::TooBigSize(self.0))?;
        let half_image_size = (self.0 / 2) as f32;
        let mut out = vec![0u8; image_size * image_size];
        out.par_iter_mut().enumerate().for_each(|(i, b)| {
            let (x, y) = (i % image_size, i / image_size);
            let vx = (x as f32) / half_image_size - 1.0;
            let vy = 1.0 - (y as f32) / half_image_size;
            *b = baseline::run(vx, vy, &p.exprs);
        });
        let elapsed = start.elapsed();
        log::info!("Thread-based Parallel Interpreter: time = {elapsed:?}");
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
    fn thread_par_16() {
        let input = read_prospero();
        assert!(input.is_ok());
        let input = input.unwrap();
        let program = parse(&input);
        assert!(program.is_ok());
        let output = ThreadParallel(16).interpret(program.unwrap());
        assert!(output.is_ok());
        let image = to_image(16, output.unwrap());
        assert!(image.is_ok());
        let png = to_png(&image.unwrap());
        assert!(png.is_ok());
        insta::assert_binary_snapshot!("thread_par_16.png", png.unwrap());
    }
}
