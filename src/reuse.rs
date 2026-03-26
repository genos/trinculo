//! Reuse previously seen expressions (similar to hash-consing or global value numbering).
use crate::{
    Translator,
    expr::{Dyad, Expr, Monad},
};
use std::{collections::HashMap, time::Instant};

/// Just a unit struct, a place to hang our [Translator] instance.
pub struct Reuse;

/// Errors that can arise when trying to translate a [`Program`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("usize {0} too big to fit as a u16")]
    TooBig(usize),
}

impl Translator for Reuse {
    type Input = Vec<Expr>;
    type Output = Vec<Expr>;
    type Error = Error;
    fn translate(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        if input.is_empty() {
            Ok(input)
        } else {
            let start = Instant::now();
            let size = input.len();
            let (mut exprs, mut ix, mut lookup) = (
                Vec::with_capacity(size),
                Vec::with_capacity(size),
                HashMap::with_capacity(size),
            );
            for e in input {
                let f = match e {
                    Expr::VarX | Expr::VarY | Expr::Const(_) => e,
                    Expr::Dyad(op, x, y) => {
                        let (a, b): (u16, u16) = (ix[usize::from(x)], ix[usize::from(y)]);
                        match op {
                            Dyad::Add | Dyad::Mul | Dyad::Max | Dyad::Min => {
                                // commutativity
                                Expr::Dyad(op, a.min(b), a.max(b))
                            }
                            Dyad::Sub => {
                                // for a - b, have we already seen b - a?
                                if let Some(&i) = lookup.get(&Expr::Dyad(Dyad::Sub, b, a)) {
                                    Expr::Monad(Monad::Neg, i)
                                } else {
                                    Expr::Dyad(Dyad::Sub, a, b)
                                }
                            }
                        }
                    }
                    Expr::Monad(op, x) => Expr::Monad(op, ix[usize::from(x)]),
                };
                if let Some(&j) = lookup.get(&f) {
                    ix.push(j);
                } else {
                    let i = u16::try_from(exprs.len()).map_err(|_| Error::TooBig(exprs.len()))?;
                    lookup.insert(f, i);
                    ix.push(i);
                    exprs.push(f);
                }
            }
            let elapsed = start.elapsed();
            let difference = size - exprs.len();
            log::info!(
                "Reuse Translator: time = {elapsed:?}, size difference = {difference} instructions"
            );
            Ok(exprs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        expr::{ProgGen, Program, parse},
        utils::read_prospero,
    };
    use chaos_theory::check;
    use rstest::*;

    #[test]
    fn reuse_shortens() {
        check(|src| {
            let p = src.any_of("prog", ProgGen);
            let o = Reuse.translate(p.exprs.clone());
            assert!(o.is_ok());
            let o = o.unwrap();
            assert!(p.len() >= o.len());
        });
    }

    #[test]
    fn reuse_idempotent() {
        check(|src| {
            let p = src.any_of("prog", ProgGen);
            let q = Reuse.translate(p.exprs);
            assert!(q.is_ok());
            let q = q.unwrap();
            let r = Reuse.translate(q.clone());
            assert!(r.is_ok());
            assert_eq!(r.unwrap(), q);
        });
    }

    #[test]
    fn reuse_on_prospero() {
        let input = read_prospero();
        assert!(input.is_ok());
        let p = parse(&input.unwrap());
        assert!(p.is_ok());
        assert!(Reuse.translate(p.unwrap().exprs).is_ok());
    }

    #[rstest]
    #[case("# empty\n", "# empty\n")]
    #[case(
        "# add comm\n_0 var-x\n_1 var-y\n_2 add _0 _1\n_3 add _1 _0",
        "# add comm\n_0 var-x\n_1 var-y\n_2 add _0 _1"
    )]
    #[case(
        "# anti-sub\n_0 var-x\n_1 var-y\n_2 sub _0 _1\n_3 sub _1 _0",
        "# anti-sub\n_0 var-x\n_1 var-y\n_2 sub _0 _1\n_3 neg _2"
    )]
    fn reuse_ok(#[case] input: &str, #[case] expected: &str) {
        let p = parse(input);
        assert!(p.is_ok());
        let p = p.unwrap();
        let q = parse(expected);
        let o = Reuse.translate(p.exprs);
        assert!(o.is_ok());
        let o = o.unwrap();
        assert!(q.is_ok());
        let q = q.unwrap();
        assert_eq!(o, q.exprs);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn reuse_too_big() {
        let n = 65536;
        let p = Program {
            header: "too big".to_string(),
            exprs: (0..=n).map(|i| Expr::Const(i as f32)).collect(),
        };
        let o = Reuse.translate(p.exprs);
        assert!(o.is_err());
        assert_eq!(o.unwrap_err(), Error::TooBig(n));
    }
}
