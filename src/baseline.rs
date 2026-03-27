//! Baseline interpretation of [`Expr`] into a vector of [u8]s.
use crate::{
    Interpreter,
    expr::{Dyad, Expr, Monad, Program},
};
use std::time::Instant;

/// Baseline interpreter; given an image size (in pixels per side), the [`Interpreter`] instance
/// will interpret the [`Expr`]s listed in a [`Program`] serially.
pub struct Baseline(pub u16);

/// Errors that can arise when interpreting a [`Program`] with a [`Baseline`] interpreter.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Interpreter for Baseline {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, input: Self::Input) -> Result<Vec<u8>, Self::Error> {
        let start = Instant::now();
        let image_size = usize::from(self.0);
        let half_image_size = f32::from(self.0 / 2);
        let mut out = vec![0u8; image_size * image_size];
        out.iter_mut().enumerate().for_each(|(i, b)| {
            let (x, y) = (i % image_size, i / image_size);
            let vx = (x as f32) / half_image_size - 1.0;
            let vy = 1.0 - (y as f32) / half_image_size;
            *b = run(vx, vy, &input.exprs);
        });
        let elapsed = start.elapsed();
        log::info!("Baseline Interpreter: time = {elapsed:?}");
        Ok(out)
    }
}

/// Sequentially run the expressions for the given input values; return a [u8] that's 255 or 0,
/// depending on whether the last computed value is less than zero.
pub(crate) fn run(vx: f32, vy: f32, xs: &[Expr]) -> u8 {
    let mut out = Vec::with_capacity(xs.len());
    for &x in xs {
        out.push(step(vx, vy, x, &out));
    }
    255 * u8::from(out.last().copied().unwrap_or_default() < 0.0)
}

/// Single step of evaluating an [`Expr`] for given input values.
pub(crate) fn step(vx: f32, vy: f32, x: Expr, out: &[f32]) -> f32 {
    match x {
        Expr::VarX => vx,
        Expr::VarY => vy,
        Expr::Const(x) => x,
        Expr::Dyad(op, x, y) => {
            let (a, b) = (out[usize::from(x)], out[usize::from(y)]);
            match op {
                Dyad::Add => a + b,
                Dyad::Sub => a - b,
                Dyad::Mul => a * b,
                Dyad::Max => a.max(b),
                Dyad::Min => a.min(b),
            }
        }
        Expr::Monad(op, x) => {
            let c = out[usize::from(x)];
            match op {
                Monad::Neg => -c,
                Monad::Square => c * c,
                Monad::Sqrt => c.sqrt(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    crate::snapshot_test!(Baseline, 64);
}
