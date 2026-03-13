//! Direct interpretation of [`expr::Expr`] into [u8].

use crate::{
    Interpreter,
    expr::{Dyad, Expr, Monad, Program},
};

pub struct Direct(pub u32);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("u32 size is too large to fit into a usize: {0}")]
    TooBigSize(u32),
    #[error("u32 size is too large to fit half of it into an f32: {0}")]
    TooBigF32(u32),
}

impl Interpreter for Direct {
    type Input = Program;
    type Error = Error;
    #[allow(clippy::cast_precision_loss)]
    fn interpret(&self, p: Program) -> Result<Vec<u8>, Error> {
        let image_size = usize::try_from(self.0).map_err(|_| Error::TooBigSize(self.0))?;
        let half_image_size = (self.0 / 2) as f32;
        let mut out = vec![0u8; image_size * image_size];
        out.iter_mut().enumerate().for_each(|(i, b)| {
            let y = i / image_size;
            let x = i % image_size;
            let vx = (x as f32) / half_image_size - 1.0;
            let vy = 1.0 - (y as f32) / half_image_size;
            *b = run(vx, vy, &p.exprs);
        });
        Ok(out)
    }
}

/// Directly apply all the expressions in sequence.
fn run(vx: f32, vy: f32, xs: &[Expr]) -> u8 {
    let mut out: Vec<f32> = Vec::with_capacity(xs.len());
    for &x in xs {
        out.push(match x {
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
        });
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
    fn direct_16() {
        let input = read_prospero();
        assert!(input.is_ok());
        let input = input.unwrap();
        let program = parse(&input);
        assert!(program.is_ok());
        let output = Direct(16).interpret(program.unwrap());
        assert!(output.is_ok());
        let image = to_image(16, output.unwrap());
        assert!(image.is_ok());
        let png = to_png(&image.unwrap());
        assert!(png.is_ok());
        insta::assert_binary_snapshot!("small.png", png.unwrap());
    }
}
