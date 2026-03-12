//! Base expressions.
use crate::Translator;
use itertools::Itertools;
use std::{
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
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

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
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

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Expr {
    VarX,
    VarY,
    Const(f32),
    Dyad { op: Dyad, x: u16, y: u16 },
    Monad { op: Monad, x: u16 },
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
            Expr::Const(c) => 0b010 | Self::from(c.to_bits()) << 32,
            Expr::Dyad { op, x, y } => {
                0b011 | (op as Self) << 3 | Self::from(x) << 32 | Self::from(y) << 48
            }
            Expr::Monad { op, x } => 0b100 | (op as Self) << 3 | Self::from(x) << 32,
        }
    }
}

impl From<&Expr> for u64 {
    fn from(x: &Expr) -> Self {
        Self::from(*x)
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
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

#[allow(clippy::cast_possible_truncation)]
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
            (0b011, 0b000, _) => Ok(Self::Dyad {
                op: Dyad::Add,
                x,
                y,
            }),
            (0b011, 0b001, _) => Ok(Self::Dyad {
                op: Dyad::Sub,
                x,
                y,
            }),
            (0b011, 0b010, _) => Ok(Self::Dyad {
                op: Dyad::Mul,
                x,
                y,
            }),
            (0b011, 0b011, _) => Ok(Self::Dyad {
                op: Dyad::Max,
                x,
                y,
            }),
            (0b011, 0b100, _) => Ok(Self::Dyad {
                op: Dyad::Min,
                x,
                y,
            }),
            (0b011, _, _) => Err(ExprU64Error::DyadOp(op)),
            (0b100, 0b000, _) if y == 0 => Ok(Self::Monad { op: Monad::Neg, x }),
            (0b100, 0b001, _) if y == 0 => Ok(Self::Monad {
                op: Monad::Square,
                x,
            }),
            (0b100, 0b010, _) if y == 0 => Ok(Self::Monad { op: Monad::Sqrt, x }),
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
            Self::Const(c) => write!(f, "const {c:?}"),
            Self::Dyad { op, x, y } => write!(f, "{op} _{x:x} _{y:x}"),
            Self::Monad { op, x } => write!(f, "{op} _{x:x}"),
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Errors that can arise when trying to parse a hex u16
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
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

/// Errors that can arise when trying to parse an expression
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
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
                let f = t
                    .parse::<f32>()
                    .map_err(|_| ExprParseError::BadF32(t.to_string()))?;
                Ok(Self::Const(f))
            }
            "add" | "sub" | "mul" | "max" | "min" => {
                let op = Dyad::from_str(t).map_err(|_| ExprParseError::BadOp(t.to_string()))?;
                let x = hex(tokens.next().ok_or(ExprParseError::DyadMissing(op))?)?;
                let y = hex(tokens.next().ok_or(ExprParseError::DyadMissing(op))?)?;
                let rest = tokens.join(" ");
                if !rest.is_empty() {
                    return Err(ExprParseError::Extra(rest));
                }
                Ok(Self::Dyad { op, x, y })
            }
            "neg" | "square" | "sqrt" => {
                let op = Monad::from_str(t).map_err(|_| ExprParseError::BadOp(t.to_string()))?;
                let x = hex(tokens.next().ok_or(ExprParseError::MonadMissing(op))?)?;
                let rest = tokens.join(" ");
                if !rest.is_empty() {
                    return Err(ExprParseError::Extra(rest));
                }
                Ok(Self::Monad { op, x })
            }
            _ => Err(ExprParseError::BadOp(t.to_string()))?,
        }
    }
}

/// A collection of base expressions, with a header string.
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[derive(PartialEq, Eq)]
pub struct Program {
    pub header: String,
    pub exprs: Vec<Expr>,
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

/// Errors that can arise when trying to parse a program
#[derive(Debug, thiserror::Error)]
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
    #[error("An error occured in trying to parse an expression at line {line}: {error}")]
    BadExpr { line: usize, error: ExprParseError },
}

impl<'a> Translator for &'a Parser {
    type Input = &'a str;
    type Output = Program;
    type Error = ParseError;
    fn translate(&self, input: &'a str) -> Result<Program, ParseError> {
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
                exprs.push(e);
            } else {
                return Err(ParseError::EmptyLine(line));
            }
        }
        Ok(Program { header, exprs })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::*;

    proptest! {
        #[test]
        fn dyad_roundtrip(x: Dyad) {
            let y = Dyad::from_str(&x.to_string());
            prop_assert!(y.is_ok());
            let y = y.unwrap();
            prop_assert_eq!(x, y);
        }

        #[test]
        fn monad_roundtrip(x: Monad) {
            let y = Monad::from_str(&x.to_string());
            prop_assert!(y.is_ok());
            let y = y.unwrap();
            prop_assert_eq!(x, y);
        }

        #[test]
        fn expr_roundtrip(x: Expr) {
            let y = Expr::from_str(&x.to_string());
            prop_assert!(y.is_ok());
            let y = y.unwrap();
            prop_assert_eq!(x, y);
        }

        #[test]
        fn expr_u64_roundtrip(x: Expr) {
            let b = u64::from(x);
            let y = Expr::try_from(b);
            prop_assert!(y.is_ok());
            let y = y.unwrap();
            prop_assert_eq!(x, y);
        }

        #[test]
        fn prog_roundtrip(xs: Program) {
            let ys = parse(&xs.to_string());
            prop_assert!(ys.is_ok());
            let ys = ys.unwrap();
            prop_assert_eq!(xs, ys);
        }
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
    #[case("neg _1 pretty please", ExprParseError::Extra("pretty please".to_string()))]
    fn expr_parse_errors(#[case] input: &str, #[case] expected: ExprParseError) {
        let x = Expr::from_str(input);
        assert!(x.is_err());
        assert_eq!(x.err().unwrap(), expected);
    }

    #[test]
    fn prospero() {
        let input = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/prospero.vm"));
        assert!(input.is_ok());
        let input = input.unwrap();
        let exprs = parse(&input);
        assert!(exprs.is_ok());
        let exprs = exprs.unwrap();
        insta::assert_snapshot!("prospero.vm", exprs);
    }
}
