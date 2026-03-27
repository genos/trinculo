//! SIMD-based parallel interpretation of [`Expr`]s.
use crate::{
    Interpreter,
    expr::{Dyad, Expr, Monad, Program},
};
use std::{
    simd::{Select, StdFloat, prelude::*},
    sync::LazyLock,
    time::Instant,
};

/// SIMD-based parallel interpreter.
///
/// Given an image size (in pixels per side), the [`Interpreter`] instance will interpret the
/// [`Expr`]s listed in a [`Program`] in parallel.
pub struct SimdParallel(pub u16);

const N: usize = 64;
static IOTA: LazyLock<Simd<usize, N>> =
    LazyLock::new(|| Simd::from_slice(&(0..N).collect::<Vec<_>>()));
const ONE: Simd<f32, N> = Simd::splat(1.0);

/// Errors that can arise when interpreting a [`Program`] with a [`SimdParallel`] interpreter.
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Interpreter for SimdParallel {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, input: Self::Input) -> Result<Vec<u8>, Self::Error> {
        let start = Instant::now();
        let image_size = usize::from(self.0);
        let mut out = vec![0u8; image_size * image_size];
        out.chunks_exact_mut(N)
            .enumerate()
            .for_each(|(i, c)| exec(i, c, &input.exprs, &Simd::splat(image_size)));
        let elapsed = start.elapsed();
        log::info!("SIMD-based Parallel Interpreter: time = {elapsed:?}");
        Ok(out)
    }
}

pub(crate) fn exec(i: usize, c: &mut [u8], exprs: &[Expr], size: &Simd<usize, N>) {
    let half_image_size = (size >> 1).cast::<f32>();
    let xy = Simd::splat(i * N) + *IOTA;
    let (x, y) = (xy % size, xy / size);
    let vx = x.cast() / half_image_size - ONE;
    let vy = ONE - y.cast() / half_image_size;
    c.copy_from_slice(&run(vx, vy, exprs).to_array());
}

/// Translation of [`crate::baseline::run`]
fn run(vx: Simd<f32, N>, vy: Simd<f32, N>, xs: &[Expr]) -> Simd<u8, N> {
    let mut out = Vec::with_capacity(xs.len());
    for &x in xs {
        out.push(step(vx, vy, x, &out));
    }
    out.last()
        .copied()
        .unwrap_or_default()
        .simd_lt(Simd::splat(0.0))
        .select(Simd::splat(255), Simd::splat(0))
}

/// Translation of [`crate::baseline::step`].
fn step(vx: Simd<f32, N>, vy: Simd<f32, N>, x: Expr, out: &[Simd<f32, N>]) -> Simd<f32, N> {
    match x {
        Expr::VarX => vx,
        Expr::VarY => vy,
        Expr::Const(x) => Simd::splat(x),
        Expr::Dyad(op, x, y) => {
            let (a, b) = (out[usize::from(x)], out[usize::from(y)]);
            match op {
                Dyad::Add => a + b,
                Dyad::Sub => a - b,
                Dyad::Mul => a * b,
                Dyad::Max => a.simd_max(b),
                Dyad::Min => a.simd_min(b),
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
    crate::snapshot_test!(SimdParallel);
}
