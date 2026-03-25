//! Reclaiming no longer used expressions, similar to [Max Bernstein's
//! approach](https://bernsteinbear.com/blog/prospero/).
use crate::{
    Interpreter, Translator, baseline,
    expr::{Expr, Program},
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

/// Expressions that are reclaimable.
#[derive(Clone, Copy)]
pub enum ExprOrDel {
    /// The original [`Expr`]
    Expr(Expr),
    /// Reclaim the given instruction.
    Delete(u16),
}

/// A program with garbage collection.
#[derive(Clone)]
pub struct ProgWithGC {
    pub header: String,
    pub exprs: HashMap<usize, ExprOrDel>,
}

///  Reclaiming translator & interpreter.
///
///  Given an image size (in pixels per side), the [`Translator`] instance will turn a
///  the [`Program`] into a [`ProgWithGC`], while [`Interpreter`] instance will interpret the
///  [`ExprOrDel`]s listed in that [`ProgWithGC`] serially.
pub struct Reclaim(pub u16);

/// Errors that can arise when trying to translate or interpret a [`Program`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {}

impl Translator for Reclaim {
    type Input = Program;
    type Output = ProgWithGC;
    type Error = Error;
    fn translate(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let start = Instant::now();
        let size = input.exprs.len();
        let (mut seen, mut with_gc) = (HashSet::with_capacity(size), Vec::with_capacity(2 * size));
        // Walk backward from end, recording the last time a monadic or dyadic expr is seen, not
        // counting the very last instruction (needless gc).
        for x in input.exprs.into_iter().rev() {
            if !with_gc.is_empty() {
                match x {
                    Expr::VarX | Expr::VarY | Expr::Const(_) => (),
                    Expr::Dyad(_, a, b) => {
                        for z in [a, b] {
                            if !seen.contains(&z) {
                                with_gc.push(ExprOrDel::Delete(z));
                            }
                            seen.insert(z);
                        }
                    }
                    Expr::Monad(_, z) => {
                        if !seen.contains(&z) {
                            with_gc.push(ExprOrDel::Delete(z));
                        }
                        seen.insert(z);
                    }
                }
            }
            with_gc.push(ExprOrDel::Expr(x));
        }
        with_gc.reverse();
        with_gc.shrink_to_fit();
        let additions = with_gc.len() - size;
        let exprs = with_gc.into_iter().enumerate().collect();
        let elapsed = start.elapsed();
        log::info!(
            "Reclaiming Translator: time = {elapsed:?}, additions = {additions} instructions"
        );
        Ok(ProgWithGC {
            header: input.header,
            exprs,
        })
    }
}

/// Follows the same setup as the [`crate::baseline::Baseline`] interpreter
impl Interpreter for Reclaim {
    type Input = ProgWithGC;
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
            *b = run(vx, vy, &mut input.exprs.clone());
        });
        let elapsed = start.elapsed();
        log::info!("Reclaiming Interpreter: time = {elapsed:?}");
        Ok(out)
    }
}

fn run(vx: f32, vy: f32, xs: &mut HashMap<usize, ExprOrDel>) -> u8 {
    let mut out = Vec::with_capacity(xs.len());
    for i in 0..xs.len() {
        match xs[&i] {
            ExprOrDel::Delete(j) => {
                xs.remove(&usize::from(j));
            }
            ExprOrDel::Expr(e) => {
                out.push(baseline::step(vx, vy, e, &out));
            }
        }
    }
    255 * u8::from(*out.last().expect("nonempty") < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        expr::parse,
        utils::{read_prospero, to_image, to_png},
    };

    #[test]
    fn reclaim_on_prospero() {
        let input = read_prospero();
        assert!(input.is_ok());
        let p = parse(&input.unwrap());
        assert!(p.is_ok());
        assert!(Reclaim(16).translate(p.unwrap()).is_ok());
    }

    #[test]
    fn reclaim_16() {
        let input = read_prospero();
        assert!(input.is_ok());
        let input = input.unwrap();
        let program = parse(&input);
        assert!(program.is_ok());
        let r = Reclaim(16);
        let prog_with_gc = r.translate(program.unwrap());
        assert!(prog_with_gc.is_ok());
        let output = Reclaim(16).interpret(prog_with_gc.unwrap());
        assert!(output.is_ok());
        let image = to_image(16, output.unwrap());
        assert!(image.is_ok());
        let png = to_png(&image.unwrap());
        assert!(png.is_ok());
        insta::assert_binary_snapshot!("reclaim_16.png", png.unwrap());
    }
}
