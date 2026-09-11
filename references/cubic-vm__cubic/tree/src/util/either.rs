use std::fmt::{self, Display};
use std::str::FromStr;

/// A value that is one of two name types. `from_str` tries `L` then `R`,
/// and the side that parses is the side that is held. When neither side
/// parses it lists both reasons as the accepted alternatives. `Display`
/// writes back whichever side is held.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: FromStr<Err = String>, R: FromStr<Err = String>> FromStr for Either<L, R> {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        L::from_str(name).map(Either::Left).or_else(|left| {
            R::from_str(name)
                .map(Either::Right)
                .map_err(|right| format!("expected either\n   a) {left}\n   b) {right}"))
        })
    }
}

impl<L: Display, R: Display> Display for Either<L, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::Left(left) => left.fmt(f),
            Either::Right(right) => right.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_is_tried_first() {
        // Both sides parse anything, so the left side must win.
        let value = Either::<Always, Always>::from_str("x").unwrap();
        assert!(matches!(value, Either::Left(_)));
    }

    #[test]
    fn test_falls_through_to_right() {
        // The left side never parses, so parsing falls through to the right.
        let value = Either::<Never, Always>::from_str("x").unwrap();
        assert!(matches!(value, Either::Right(_)));
    }

    #[test]
    fn test_lists_both_errors_when_neither_side_parses() {
        let error = Either::<Never, Never>::from_str("x").unwrap_err();
        assert_eq!(error, "expected either\n   a) no parse\n   b) no parse");
    }

    #[derive(Debug)]
    struct Always;
    #[derive(Debug)]
    struct Never;

    impl FromStr for Always {
        type Err = String;
        fn from_str(_: &str) -> Result<Self, String> {
            Ok(Always)
        }
    }

    impl FromStr for Never {
        type Err = String;
        fn from_str(_: &str) -> Result<Self, String> {
            Err("no parse".to_string())
        }
    }
}
