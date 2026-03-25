//! Removing unused expressions (A.K.A. dead code elimination).
use crate::{
    Translator,
    expr::{Expr, Program},
};
use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

/// Just a unit struct, a place to hang our [Translator] instance.
pub struct Unused;

/// Errors that can arise when trying to translate a [`Program`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("Empty program")]
    Empty,
}

impl Translator for Unused {
    type Input = Program;
    type Output = Program;
    type Error = Error;
    fn translate(&self, prog: Program) -> Result<Program, Error> {
        if prog.is_empty() {
            Ok(prog)
        } else {
            let start = Instant::now();
            let size = prog.exprs.len();
            let (mut unused, mut queue) = ((0..size - 1).collect::<HashSet<_>>(), VecDeque::new());
            for a in prog.exprs.last().ok_or(Error::Empty)?.args() {
                queue.push_back(a);
            }
            while let Some(i) = queue.pop_front() {
                unused.remove(&usize::from(i));
                prog.exprs[usize::from(i)]
                    .args()
                    .for_each(|a| queue.push_back(a));
            }
            let mut exprs = prog.exprs.clone();
            for &i in unused.iter() {
                for e in &mut exprs[i..] {
                    match e {
                        Expr::VarX | Expr::VarY | Expr::Const(_) => (),
                        Expr::Dyad(_, x, y) => {
                            if usize::from(*x) > i {
                                *x -= 1;
                            }
                            if usize::from(*y) > i {
                                *y -= 1;
                            }
                        }
                        Expr::Monad(_, x) => {
                            if usize::from(*x) > i {
                                *x -= 1;
                            }
                        }
                    }
                }
            }
            let mut remove = unused.iter().collect::<Vec<_>>();
            remove.sort_unstable();
            remove.reverse();
            for &i in remove {
                exprs.remove(i);
            }
            let difference = size - exprs.len();
            let elapsed = start.elapsed();
            log::info!(
                "Unused Translator: time = {elapsed:?}, size difference = {difference} instructions"
            );
            Ok(Program {
                header: prog.header,
                exprs,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{expr::parse, utils::read_prospero};
    use proptest::prelude::*;
    use rstest::*;

    proptest! {

        #[test]
        fn unused_shortens(p: Program) {
            let o = Unused.translate(p.clone());
            prop_assert!(o.is_ok());
            let o = o.unwrap();
            prop_assert!(p.len() >= o.len());
        }

    }

    #[rstest]
    #[case("# empty\n", "# empty\n")]
    #[case(
        "# unused y\n_0 var-x\n_1 var-y\n_2 neg _0",
        "# unused y\n_0 var-x\n_1 neg _0"
    )]
    fn unused_ok(#[case] input: &str, #[case] expected: &str) {
        let p = parse(input);
        assert!(p.is_ok());
        let p = p.unwrap();
        let q = parse(expected);
        let o = Unused.translate(p);
        assert!(o.is_ok());
        let o = o.unwrap();
        assert!(q.is_ok());
        let q = q.unwrap();
        assert_eq!(o, q);
    }

    #[test]
    fn unused_on_prospero() {
        let input = read_prospero();
        assert!(input.is_ok());
        let p = parse(&input.unwrap());
        assert!(p.is_ok());
        assert!(Unused.translate(p.unwrap()).is_ok());
    }
}
