//! Reuse previously seen expressions (similar to hash-consing or global value numbering)

use crate::{
    Translator,
    expr::{Dyad, Expr, Monad, Program},
};
use std::collections::HashMap;

pub struct Reuse;
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Translator for Reuse {
    type Input = Program;
    type Output = Program;
    type Error = Error;
    fn translate(&self, p: Program) -> Result<Program, Error> {
        let size = p.exprs.len();
        let (mut exprs, mut ix, mut lookup, mut i) = (
            Vec::with_capacity(size),
            Vec::with_capacity(size),
            HashMap::<Expr, u16>::with_capacity(size),
            0u16,
        );
        for e in p.exprs {
            let f = match e {
                Expr::VarX | Expr::VarY | Expr::Const(_) | Expr::Monad(_, _) => e,
                Expr::Dyad(op, x, y) => match op {
                    Dyad::Add | Dyad::Mul | Dyad::Max | Dyad::Min => {
                        // commutativity
                        Expr::Dyad(op, x.min(y), x.max(y))
                    }
                    Dyad::Sub => {
                        // for x - y, have we already seen y - x?
                        if let Some(&i) = lookup.get(&Expr::Dyad(Dyad::Sub, y, x)) {
                            Expr::Monad(Monad::Neg, i)
                        } else {
                            e
                        }
                    }
                },
            };
            if let Some(&j) = lookup.get(&f) {
                ix.push(j);
            } else {
                lookup.insert(f, i);
                ix.push(i);
                exprs.push(f);
                i += 1;
            }
        }
        Ok(Program {
            header: p.header,
            exprs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parse;
    use proptest::prelude::*;
    use rstest::*;

    proptest! {

        #[test]
        fn reuse_shortens(p: Program) {
            let o = Reuse.translate(p.clone());
            prop_assert!(o.is_ok());
            let o = o.unwrap();
            prop_assert_eq!(p.is_empty(), o.is_empty());
            prop_assert!(p.len() >= o.len());
        }

    }

    #[rstest]
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
        let o = Reuse.translate(p);
        assert!(o.is_ok());
        let o = o.unwrap();
        assert!(q.is_ok());
        let q = q.unwrap();
        assert_eq!(o, q);
    }
}
