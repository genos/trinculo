//! Thread-based parallel interpretation of [`crate::expr::Expr`]s.
use crate::{Interpreter, baseline, expr::Program};
use rayon::prelude::*;
use std::time::Instant;

/// Thread-based parallel interpreter.
///
/// Given an image size (in pixels per side), the [`Interpreter`] instance will interpret the
/// [`crate::expr::Expr`]s listed in a [`Program`] in parallel.
pub struct ThreadParallel(pub u16);

/// Errors that can arise when interpreting a [`Program`] with a [`ThreadParallel`] interpreter.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

/// Reuses [`crate::baseline`]'s internal `run()` function for any given pixel.
impl Interpreter for ThreadParallel {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, input: Self::Input) -> Result<Vec<u8>, Self::Error> {
        let start = Instant::now();
        let image_size = usize::from(self.0);
        let half_image_size = f32::from(self.0 / 2);
        let mut out = vec![0u8; image_size * image_size];
        out.par_iter_mut().enumerate().for_each(|(i, b)| {
            let (x, y) = (i % image_size, i / image_size);
            let vx = (x as f32) / half_image_size - 1.0;
            let vy = 1.0 - (y as f32) / half_image_size;
            *b = baseline::run(vx, vy, &input.exprs);
        });
        let elapsed = start.elapsed();
        log::info!("Thread-based Parallel Interpreter: time = {elapsed:?}");
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    crate::snapshot_test!(ThreadParallel);
}
