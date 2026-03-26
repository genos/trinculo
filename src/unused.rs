//! Removing unused expressions (A.K.A. dead code elimination).
use crate::{Translator, expr::Expr};
use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

/// Just a unit struct, a place to hang our [Translator] instance.
pub struct Unused;

/// Errors that can arise when trying to translate a [`Program`].
#[derive(Debug, thiserror::Error)]
pub enum Error {}

impl Translator for Unused {
    type Input = Vec<Expr>;
    type Output = Vec<Expr>;
    type Error = Error;
    fn translate(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        if input.is_empty() {
            Ok(input)
        } else {
            let start = Instant::now();
            let size = input.len();
            let (mut unused, mut q) = ((0..size - 1).collect::<HashSet<_>>(), VecDeque::new());
            for a in input.last().expect("Nonempty on this branch").args() {
                q.push_back(usize::from(a));
            }
            while let Some(i) = q.pop_front() {
                unused.remove(&i);
                input[i].args().for_each(|a| q.push_back(usize::from(a)));
            }
            let mut exprs = input;
            for &i in &unused {
                for e in &mut exprs[i..] {
                    match e {
                        Expr::VarX | Expr::VarY | Expr::Const(_) => (),
                        Expr::Dyad(_, x, y) => {
                            *x -= u16::from(usize::from(*x) > i);
                            *y -= u16::from(usize::from(*y) > i);
                        }
                        Expr::Monad(_, x) => *x -= u16::from(usize::from(*x) > i),
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
            Ok(exprs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        expr::{Program, parse},
        utils::read_prospero,
    };
    use chaos_theory::check;
    use rstest::*;

    #[test]
    fn unused_shortens() {
        check(|src| {
            let p = src.any::<Program>("prog");
            let o = Unused.translate(p.clone().exprs);
            assert!(o.is_ok());
            let o = o.unwrap();
            assert!(p.len() >= o.len());
        });
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
        let o = Unused.translate(p.exprs);
        assert!(o.is_ok());
        let o = o.unwrap();
        assert!(q.is_ok());
        let q = q.unwrap();
        assert_eq!(o, q.exprs);
    }

    #[test]
    fn unused_on_prospero() {
        let input = read_prospero();
        assert!(input.is_ok());
        let p = parse(&input.unwrap());
        assert!(p.is_ok());
        assert!(Unused.translate(p.unwrap().exprs).is_ok());
    }
}
