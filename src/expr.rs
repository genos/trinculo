//! Expressions and Programs made out of them.
use crate::Translator;
use std::{
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
    time::Instant,
};

/// A dyadic (A.K.A. binary) operation.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
pub enum Dyad {
    Add,
    Sub,
    Mul,
    Max,
    Min,
}
impl fmt::Debug for Dyad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// A monadic (A.K.A. unary) operation.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
pub enum Monad {
    Neg,
    Square,
    Sqrt,
}
impl fmt::Debug for Monad {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// An expression is either:
/// - the input variable _x_,
/// - the input variable _y_,
/// - a constant floating point value,
/// - a [`Dyad`] (A.K.A. binary operation),
/// - or a [`Monad`] (A.K.A. unary operation).
///
/// For the latter two, the [u16]s correspond to an index referring to a previous expression in a
/// [`Program`].
///
/// Because an [`Expr`] is small enough to fit into 64 bits, and to sidestep the ordering,
/// equality, and other issues associated with floating point values, all of the equality,
/// ordering, and hashing traits for an [`Expr`] are based off of its representation as a [u64] via
/// the `<u64 as From<Expr>>::from` implementation.
#[derive(Clone, Copy)]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
pub enum Expr {
    VarX,
    VarY,
    Const(f32),
    Dyad(Dyad, u16, u16),
    Monad(Monad, u16),
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        u64::from(self) == u64::from(other)
    }
}
impl Eq for Expr {}
impl Hash for Expr {
    fn hash<H: Hasher>(&self, h: &mut H) {
        h.write_u64(u64::from(self));
    }
}

impl Expr {
    pub(crate) fn args(&self) -> impl Iterator<Item = u16> {
        match self {
            // Couldn't find a way to make this work nicely with iterators; type inference
            // bit me.
            Self::VarX | Self::VarY | Self::Const(_) => vec![].into_iter(),
            Self::Dyad(_, x, y) => vec![*x, *y].into_iter(),
            Self::Monad(_, x) => vec![*x].into_iter(),
        }
    }
}

/// Packing an [Expr] into a [u64].
///
/// The binary format is as follows:
/// ```text
/// | 3 bit tag | 3 bit op | 26 zero bits (junk) | 32 bit payload |
/// ```
/// where
/// - The `tag` describes which variant of the [Expr] we have:
///     - `0b000 => ` [`Expr::VarX`],
///     - `0b001 => ` [`Expr::VarY`],
///     - `0b010 => ` [`Expr::Const`],
///     - `0b011 => ` [`Expr::Dyad`],
///     - `0b100 => ` [`Expr::Monad`],
///     - any other value is an **error**.
/// - The `op` _either_:
///     - describes the [Dyad] operation, if the `tag` is the dyad value of `0b011`:
///         - `0b000 => ` [`Dyad::Add`],
///         - `0b001 => ` [`Dyad::Sub`],
///         - `0b010 => ` [`Dyad::Mul`],
///         - `0b011 => ` [`Dyad::Max`],
///         - `0b100 => ` [`Dyad::Min`],
///         - any other value is an **error**.
///     - describes the [Monad] operation, if the `tag` is the monad value of `0b100`:
///         - `0b000 => ` [`Monad::Neg`],
///         - `0b001 => ` [`Monad::Square`],
///         - `0b010 => ` [`Monad::Sqrt`],
///         - any other value is an **error**.
///     - is zero otherwise (any other value is an **error**).
/// - The "junk" bits are zero, and it's an **error** to have them otherwise.
/// - The 32 bit payload is _either_:
///     - the 32 bits of the [f32] in [`Expr::Const`], if the `tag` is `0b010`,
///     - `(x | y << 16)`, where `x` and `y` are the [u16] arguments of [`Expr::Dyad`], if `tag` is
///       `0b011` and `op` is an appropriate value,
///     - `x`, where `x` is the [u16] argument of [`Expr::Monad`], if `tag` is `0b100` and `op` is
///       an appropriate value (note that the top 16 bits of `payload` **must** be zero in this
///       case),
///     - is zero otherwise (any other value is an **error**).
///
/// Having "junk" bits is less than ideal, but I couldn't see a way to encode the `tag`, `op`, and
/// `payload` without needing 64 bits—the `payload` is often 32 bits on its own.
impl From<Expr> for u64 {
    fn from(x: Expr) -> Self {
        match x {
            Expr::VarX => 0b000,
            Expr::VarY => 0b001,
            Expr::Const(x) => 0b010 | Self::from(x.to_bits()) << 32,
            Expr::Dyad(op, x, y) => {
                0b011 | (op as Self) << 3 | Self::from(x) << 32 | Self::from(y) << 48
            }
            Expr::Monad(op, x) => 0b100 | (op as Self) << 3 | Self::from(x) << 32,
        }
    }
}

impl From<&Expr> for u64 {
    fn from(x: &Expr) -> Self {
        Self::from(*x)
    }
}

/// Errors that can arise when trying to parse a [u64] as an [`Expr`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExprU64Error {
    #[error("Junk bits should be zero: {0:b}")]
    Junk(u64),
    #[error("Nonzero bits in VarX op or payload section: {0:b}")]
    NzX(u64),
    #[error("Nonzero bits in VarY op or payload section: {0:b}")]
    NzY(u64),
    #[error("Nonzero bits in Const op or payload section: {0:b}")]
    NzC(u64),
    #[error("Unrecognized dyadic op: {0:b}")]
    DyadOp(u64),
    #[error("Nonzero bits in monadic y section: {0:b}")]
    NzM(u64),
    #[error("Unrecognized monadic op: {0:b}")]
    MonadOp(u64),
    #[error("Unrecognized tag: {0:b}")]
    Tag(u64),
}

impl TryFrom<u64> for Expr {
    type Error = ExprU64Error;
    fn try_from(n: u64) -> Result<Self, Self::Error> {
        // middle 26 bits empty by design
        let junk = (n >> 6) & 0x3ff_ffff;
        if junk != 0 {
            return Err(ExprU64Error::Junk(junk));
        }
        let tag = n & 0b111;
        let op = (n >> 3) & 0b111;
        let payload = (n >> 32) as u32;
        let x = (payload & 0xffff) as u16;
        let y = (payload >> 16) as u16;
        match (tag, op, payload) {
            (0b000, 0, 0) => Ok(Self::VarX),
            (0b000, _, _) => Err(ExprU64Error::NzX((op << 3) | (u64::from(payload) << 32))),
            (0b001, 0, 0) => Ok(Self::VarY),
            (0b001, _, _) => Err(ExprU64Error::NzY((op << 3) | (u64::from(payload) << 32))),
            (0b010, 0, _) => Ok(Self::Const(f32::from_bits(payload))),
            (0b010, _, _) => Err(ExprU64Error::NzC((op << 3) | (u64::from(payload) << 32))),
            (0b011, 0b000, _) => Ok(Self::Dyad(Dyad::Add, x, y)),
            (0b011, 0b001, _) => Ok(Self::Dyad(Dyad::Sub, x, y)),
            (0b011, 0b010, _) => Ok(Self::Dyad(Dyad::Mul, x, y)),
            (0b011, 0b011, _) => Ok(Self::Dyad(Dyad::Max, x, y)),
            (0b011, 0b100, _) => Ok(Self::Dyad(Dyad::Min, x, y)),
            (0b011, _, _) => Err(ExprU64Error::DyadOp(op << 3)),
            (0b100, 0b000, _) if y == 0 => Ok(Self::Monad(Monad::Neg, x)),
            (0b100, 0b001, _) if y == 0 => Ok(Self::Monad(Monad::Square, x)),
            (0b100, 0b010, _) if y == 0 => Ok(Self::Monad(Monad::Sqrt, x)),
            (0b100, _, _) if y == 0 => Err(ExprU64Error::MonadOp(op << 3)),
            (0b100, _, _) => Err(ExprU64Error::NzM(u64::from(y) << 48)),
            _ => Err(ExprU64Error::Tag(tag)),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarX => f.write_str("var-x"),
            Self::VarY => f.write_str("var-y"),
            Self::Const(x) => write!(f, "const {x:?}"),
            Self::Dyad(op, x, y) => write!(f, "{op} _{x:x} _{y:x}"),
            Self::Monad(op, x) => write!(f, "{op} _{x:x}"),
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Errors that can arise when trying to parse a hex [u16].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HexParseError {
    #[error("Hex value missing leading underscore: {0}")]
    Underscore(String),
    #[error("{0} is not a valid hex value")]
    NotHex(String),
    #[error("Hex value {0:x} is too large to fit in a u16")]
    TooBig(usize),
}

fn hex(t: &str) -> Result<u16, HexParseError> {
    let j = usize::from_str_radix(
        t.strip_prefix('_')
            .ok_or_else(|| HexParseError::Underscore(t.to_string()))?,
        16,
    )
    .map_err(|_| HexParseError::NotHex(t.to_string()))?;
    u16::try_from(j).map_err(|_| HexParseError::TooBig(j))
}

/// Errors that can arise when trying to parse an expression.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExprParseError {
    #[error("Missing an operation")]
    MissingOp,
    #[error("const is missing its value")]
    ConstValue,
    #[error("Unable to parse f32 from {0}")]
    BadF32(String),
    #[error("Dyadic operator {0} is missing an operand")]
    DyadMissing(Dyad),
    #[error("Monadic operator {0} is missing its operand")]
    MonadMissing(Monad),
    #[error("Error parsing a hex value: {0}")]
    BadHex(#[from] HexParseError),
    #[error("Unknown operation: {0}")]
    BadOp(String),
    #[error("Extra tokens: {0}")]
    Extra(String),
}

impl FromStr for Expr {
    type Err = ExprParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.split_ascii_whitespace();
        let t = tokens.next().ok_or(ExprParseError::MissingOp)?;
        match t {
            "var-x" => Ok(Self::VarX),
            "var-y" => Ok(Self::VarY),
            "const" => {
                let t = tokens.next().ok_or(ExprParseError::ConstValue)?;
                let x = t
                    .parse::<f32>()
                    .map_err(|_| ExprParseError::BadF32(t.to_string()))?;
                Ok(Self::Const(x))
            }
            "add" | "sub" | "mul" | "max" | "min" => {
                let Ok(op) = Dyad::from_str(t) else {
                    unreachable!("add|sub|mul|max|min: {t}")
                };
                let x = hex(tokens.next().ok_or(ExprParseError::DyadMissing(op))?)?;
                let y = hex(tokens.next().ok_or(ExprParseError::DyadMissing(op))?)?;
                let rest = tokens.collect::<Vec<_>>().join(" ");
                if !rest.is_empty() {
                    return Err(ExprParseError::Extra(rest));
                }
                Ok(Self::Dyad(op, x, y))
            }
            "neg" | "square" | "sqrt" => {
                let Ok(op) = Monad::from_str(t) else {
                    unreachable!("neg|square|sqrt: {t}")
                };
                let x = hex(tokens.next().ok_or(ExprParseError::MonadMissing(op))?)?;
                let rest = tokens.collect::<Vec<_>>().join(" ");
                if !rest.is_empty() {
                    return Err(ExprParseError::Extra(rest));
                }
                Ok(Self::Monad(op, x))
            }
            _ => Err(ExprParseError::BadOp(t.to_string()))?,
        }
    }
}

/// A collection of [`Expr`] expressions, in order, with a header string.
#[derive(PartialEq, Eq, Clone)]
pub struct Program {
    pub header: String,
    pub exprs: Vec<Expr>,
}

#[cfg(test)]
#[derive(Debug)]
struct ProgGen;
#[cfg(test)]
impl chaos_theory::Generator for ProgGen {
    type Item = Program;
    #[allow(clippy::many_single_char_names)]
    fn next(&self, src: &mut chaos_theory::SourceRaw, example: Option<&Self::Item>) -> Self::Item {
        use chaos_theory::{Arbitrary, Effect, make};
        let header = src.any_of(
            "header",
            make::string_matching(r"#( \w)+", true),
            example.map(|e| &e.header),
        );
        let exprs = src
            .repeat(
                "exprs",
                example.map(|e| e.exprs.iter()),
                ..(u16::MAX as usize),
                Vec::with_capacity,
                |xs, src, ex| {
                    let c = Expr::Const(f32::arbitrary().next(src, Some(&0.0)));
                    let n = u16::try_from(xs.len()).expect("< u16::MAX by design");
                    xs.push(match xs.len() {
                        0 => src.any_of("<0>", make::one_of(&[Expr::VarX, Expr::VarY, c]), ex),
                        1 => {
                            let m = Expr::Monad(Monad::arbitrary().next(src, None), 0);
                            src.any_of("<1>", make::one_of(&[Expr::VarX, Expr::VarY, c, m]), ex)
                        }
                        _ => {
                            let x = make::int_in_range(..n).next(src, Some(&0));
                            let y = make::int_in_range(..n).next(src, Some(&0));
                            let m = Expr::Monad(Monad::arbitrary().next(src, None), x);
                            let d = Expr::Dyad(Dyad::arbitrary().next(src, None), x, y);
                            src.any_of("<∞>", make::one_of(&[Expr::VarX, Expr::VarY, c, m, d]), ex)
                        }
                    });
                    Effect::Success
                },
            )
            .unwrap_or_default();
        Program { header, exprs }
    }
}
#[cfg(test)]
impl chaos_theory::Arbitrary for Program {
    fn arbitrary() -> impl chaos_theory::Generator<Item = Self> {
        ProgGen
    }
}

impl Program {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.exprs.len()
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.exprs.is_empty()
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "# {}", self.header)?;
        for (i, x) in self.exprs.iter().enumerate() {
            writeln!(f, "_{i:x} {x}")?;
        }
        Ok(())
    }
}
impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Parse a string into a program using an internal [Translator] called a [Parser].
///
/// # Errors
/// If something goes wrong during parsing.
pub fn parse(s: &str) -> Result<Program, ParseError> {
    (&Parser).translate(s)
}

/// Just a unit struct, a place to hang our [Translator] instance.
pub struct Parser;

/// Errors that can arise when trying to [`parse`] a [`Program`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("Empty program")]
    EmptyProg,
    #[error("Malformed header: {0}")]
    BadHeader(String),
    #[error("Empty line: {0}")]
    EmptyLine(usize),
    #[error("Unable to parse hex u16 at line {line}: {error}")]
    BadHex { line: usize, error: HexParseError },
    #[error("Incorrect line number at line {line}: found {value}")]
    LineNumber { line: usize, value: u16 },
    #[error("An error occurred in trying to parse an expression at line {line}: {error}")]
    BadExpr { line: usize, error: ExprParseError },
    #[error("Expression {expr} references future or self-referential arguments at line {line}")]
    FutureOrSelfArgs { line: usize, expr: Expr },
}

impl<'a> Translator for &'a Parser {
    type Input = &'a str;
    type Output = Program;
    type Error = ParseError;
    fn translate(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let start = Instant::now();
        let mut exprs = Vec::new();
        let mut lines = input.lines();
        let header = lines.next().ok_or(ParseError::EmptyProg)?.to_string();
        let header = header
            .strip_prefix("# ")
            .ok_or_else(|| ParseError::BadHeader(header.clone()))?
            .to_string();
        for (line, s) in lines.enumerate() {
            if let Some((t, rest)) = s.split_once(|c| char::is_ascii_whitespace(&c)) {
                let j = hex(t).map_err(|error| ParseError::BadHex { line, error })?;
                if usize::from(j) != line {
                    return Err(ParseError::LineNumber { line, value: j });
                }
                let e =
                    Expr::from_str(rest).map_err(|error| ParseError::BadExpr { line, error })?;
                for arg in e.args() {
                    if usize::from(arg) >= line {
                        return Err(ParseError::FutureOrSelfArgs { line, expr: e });
                    }
                }
                exprs.push(e);
            } else {
                return Err(ParseError::EmptyLine(line));
            }
        }
        let elapsed = start.elapsed();
        log::info!("Parsing: time = {elapsed:?}");
        Ok(Program { header, exprs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaos_theory::{check, make::int_in_range};
    use rstest::*;
    use std::collections::HashSet;

    #[test]
    fn dyad_roundtrip() {
        check(|src| {
            let x = src.any::<Dyad>("dyad");
            let y = Dyad::from_str(&x.to_string());
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn dyad_roundtrip_dbg() {
        check(|src| {
            let x = src.any::<Dyad>("dyad");
            let y = Dyad::from_str(&format!("{x:?}"));
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn monad_roundtrip() {
        check(|src| {
            let x = src.any::<Monad>("monad");
            let y = Monad::from_str(&x.to_string());
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn monad_roundtrip_dbg() {
        check(|src| {
            let x = src.any::<Monad>("monad");
            let y = Monad::from_str(&format!("{x:?}"));
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn expr_roundtrip() {
        check(|src| {
            let x = src.any::<Expr>("expr");
            let y = Expr::from_str(&x.to_string());
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn expr_roundtrip_dbg() {
        check(|src| {
            let x = src.any::<Expr>("expr");
            let y = Expr::from_str(&format!("{x:?}"));
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn expr_hash_eq() {
        check(|src| {
            let x = src.any::<Expr>("Expr x");
            let y = src.any::<Expr>("Expr y");
            assert_eq!(x == y, HashSet::from([x]) == HashSet::from([y]));
        });
    }

    #[test]
    fn expr_u64_roundtrip() {
        check(|src| {
            let x = src.any::<Expr>("expr");
            let b = u64::from(x);
            let y = Expr::try_from(b);
            assert!(y.is_ok());
            let y = y.unwrap();
            assert_eq!(x, y);
        });
    }

    #[test]
    fn expr_u64_junk() {
        check(|src| {
            let tag = src.any_of("tag", int_in_range(0u64..5));
            let junk = src.any_of("junk", int_in_range(1u64..(1 << 26)));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            let x = Expr::try_from(tag | (junk << 6) | (payload << 32));
            assert_eq!(x, Err(ExprU64Error::Junk(junk)));
        });
    }

    #[test]
    fn expr_u64_nzx() {
        check(|src| {
            let op = src.any_of("op", int_in_range(0u64..(1 << 3)));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            if (op, payload) != (0, 0) {
                let x = Expr::try_from((op << 3) | (payload << 32));
                assert_eq!(x, Err(ExprU64Error::NzX((op << 3) | (payload << 32))));
            }
        });
    }

    #[test]
    fn expr_u64_nzy() {
        check(|src| {
            let op = src.any_of("op", int_in_range(0u64..(1 << 3)));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            if (op, payload) != (0, 0) {
                let x = Expr::try_from(1 | (op << 3) | (payload << 32));
                assert_eq!(x, Err(ExprU64Error::NzY((op << 3) | (payload << 32))));
            }
        });
    }

    #[test]
    fn expr_u64_nzc() {
        check(|src| {
            let op = src.any_of("op", int_in_range(1u64..(1 << 3)));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            let x = Expr::try_from(2 | (op << 3) | (payload << 32));
            assert_eq!(x, Err(ExprU64Error::NzC((op << 3) | (payload << 32))));
        });
    }

    #[test]
    fn expr_u64_dyadop() {
        check(|src| {
            let op = src.any_of("op", int_in_range(5u64..(1 << 3)));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            let x = Expr::try_from(3 | (op << 3) | (payload << 32));
            assert_eq!(x, Err(ExprU64Error::DyadOp(op << 3)));
        });
    }

    #[test]
    fn expr_u64_monadop() {
        check(|src| {
            let op = src.any_of("op", int_in_range(3u64..(1 << 3)));
            let x = src.any_of("x", int_in_range(0u64..(1 << 16)));
            let x = Expr::try_from(4 | (op << 3) | (x << 32));
            assert_eq!(x, Err(ExprU64Error::MonadOp(op << 3)));
        });
    }

    #[test]
    fn expr_u64_nzm() {
        check(|src| {
            let op = src.any_of("op", int_in_range(0u64..3));
            let x = src.any_of("x", int_in_range(0u64..(1 << 16)));
            let y = src.any_of("y", int_in_range(1u64..(1 << 16)));
            let x = Expr::try_from(4 | (op << 3) | (x << 32) | (y << 48));
            assert_eq!(x, Err(ExprU64Error::NzM(y << 48)));
        });
    }

    #[test]
    fn exp_u64_tag() {
        check(|src| {
            let tag = src.any_of("tag", int_in_range(6u64..8));
            let op = src.any_of("op", int_in_range(0u64..3));
            let payload = src.any_of("payload", int_in_range(0u64..(1 << 32)));
            let x = Expr::try_from(tag | (op << 3) | (payload << 32));
            assert_eq!(x, Err(ExprU64Error::Tag(tag)));
        });
    }

    #[test]
    fn prog_roundtrip() {
        check(|src| {
            let p = src.any::<Program>("prog");
            let q = parse(&p.to_string());
            assert!(q.is_ok());
            let q = q.unwrap();
            assert_eq!(p, q);
        });
    }

    #[test]
    fn prog_roundtrip_dbg() {
        check(|src| {
            let p = src.any::<Program>("prog");
            let q = parse(&format!("{p:?}"));
            assert!(q.is_ok());
            let q = q.unwrap();
            assert_eq!(p, q);
        });
    }

    #[test]
    fn prog_empty_len_match() {
        check(|src| {
            let p = src.any::<Program>("prog");
            assert_eq!(p.is_empty(), p.len() == 0);
        });
    }

    #[rstest]
    #[case("0", HexParseError::Underscore("0".to_string()))]
    #[case("_q", HexParseError::NotHex("_q".to_string()))]
    #[case("_10000", HexParseError::TooBig(0x10000))]
    fn hex_parse_errors(#[case] input: &str, #[case] expected: HexParseError) {
        let h = hex(input);
        assert!(h.is_err());
        assert_eq!(h.err().unwrap(), expected);
    }

    #[rstest]
    #[case("", ExprParseError::MissingOp)]
    #[case("const", ExprParseError::ConstValue)]
    #[case("add _1", ExprParseError::DyadMissing(Dyad::Add))]
    #[case("sub", ExprParseError::DyadMissing(Dyad::Sub))]
    #[case("neg", ExprParseError::MonadMissing(Monad::Neg))]
    #[case("sqrt", ExprParseError::MonadMissing(Monad::Sqrt))]
    #[case("mul _1 0", ExprParseError::BadHex(HexParseError::Underscore("0".to_string())))]
    #[case("max _q _1", ExprParseError::BadHex(HexParseError::NotHex("_q".to_string())))]
    #[case(
        "square _10000",
        ExprParseError::BadHex(HexParseError::TooBig(0x10000))
    )]
    #[case("transmogrify _1 _2", ExprParseError::BadOp("transmogrify".to_string()))]
    #[case("const asdf", ExprParseError::BadF32("asdf".to_string()))]
    #[case("min _1 _2 pretty please", ExprParseError::Extra("pretty please".to_string()))]
    #[case("neg _1 pretty please", ExprParseError::Extra("pretty please".to_string()))]
    fn expr_parse_errors(#[case] input: &str, #[case] expected: ExprParseError) {
        let x = Expr::from_str(input);
        assert!(x.is_err());
        assert_eq!(x.err().unwrap(), expected);
    }

    #[rstest]
    #[case("", ParseError::EmptyProg)]
    #[case("test\n_0 var-x", ParseError::BadHeader("test".to_string()))]
    #[case("# test\n_q var-x", ParseError::BadHex{line: 0, error: HexParseError::NotHex("_q".to_string())})]
    #[case("# test\n_1 var-x", ParseError::LineNumber{line: 0, value: 1})]
    #[case("# test\n_0 nonsense", ParseError::BadExpr{line: 0, error: ExprParseError::BadOp("nonsense".to_string())})]
    #[case("# test\n_1", ParseError::EmptyLine(0))]
    #[case("# test\n_0 add _1 _2", ParseError::FutureOrSelfArgs{line: 0, expr: Expr::Dyad(Dyad::Add, 1, 2)})]
    fn prog_parse_errors(#[case] input: &str, #[case] expected: ParseError) {
        let p = parse(input);
        assert!(p.is_err());
        assert_eq!(p.err().unwrap(), expected);
    }

    #[test]
    fn prospero() {
        let input = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/prospero.vm"));
        assert!(input.is_ok());
        let input = input.unwrap();
        let prog = parse(&input);
        assert!(prog.is_ok());
        let prog = prog.unwrap();
        insta::assert_snapshot!("prospero.vm", prog);
    }
}
