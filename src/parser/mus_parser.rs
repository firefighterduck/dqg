//! Parser for the output of picomus and similar MUS solver.

use crate::Error;

pub type BinInput<'a> = &'a [u8];
pub type BinParseError<'a> = nom::error::VerboseError<BinInput<'a>>;
pub type BinParseResult<'a, O> = nom::IResult<BinInput<'a>, O, BinParseError<'a>>;

fn _parse_comment(input: BinInput<'_>) -> BinParseResult<'_, ()> {
    use nom::{
        character::complete::{char, line_ending, not_line_ending},
        combinator::value,
        error::context,
        multi::many_till,
        sequence::terminated,
    };

    context(
        "MUS Comment line",
        value(
            (),
            terminated(char('c'), many_till(not_line_ending, line_ending)),
        ),
    )(input)
}

fn _parse_unsat(input: BinInput<'_>) -> BinParseResult<'_, ()> {
    use nom::{
        bytes::complete::tag, character::complete::line_ending, combinator::value, error::context,
        sequence::pair,
    };

    context(
        "MUS UNSAT result line",
        value((), pair(tag("s UNSATISFIABLE"), line_ending)),
    )(input)
}

fn _parse_clause_number(input: BinInput<'_>) -> BinParseResult<'_, usize> {
    use nom::{
        bytes::complete::tag,
        character::complete::{line_ending, u64},
        combinator::map,
        error::context,
        sequence::{preceded, terminated},
    };

    context(
        "MUS core clause line",
        map(
            terminated(preceded(tag("v "), u64), line_ending),
            |clause| clause as usize,
        ),
    )(input)
}

/// Parse output of picomus and return core as clause indices.
pub fn _parse_mus(input: BinInput<'_>) -> Result<Vec<usize>, Error> {
    use nom::{
        branch::alt,
        combinator::eof,
        error::context,
        multi::{fold_many0, many1},
    };

    let uninteresting = alt((_parse_comment, _parse_unsat));
    let mut skip = context(
        "Comment and UNSAT lines",
        fold_many0(uninteresting, || (), |_, _| ()),
    );

    let mut core_clauses = context("Clauses in core", many1(_parse_clause_number));

    let (res, _) = skip(input)?;
    let (res, mut core) = core_clauses(res)?;
    eof::<BinInput<'_>, BinParseError<'_>>(res)?;

    let last = core.pop();
    assert_eq!(
        last,
        Some(0),
        "Last core clause not 0! Picomus output not complete!"
    );
    Ok(core)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_comment() -> Result<(), Error> {
        let comment = b"c whatever is written here, I don't really care lul \n";

        _parse_comment(comment)?;

        Ok(())
    }

    #[test]
    fn test_parse_unsat() -> Result<(), Error> {
        let unsat = b"s UNSATISFIABLE\n";

        _parse_unsat(unsat)?;

        Ok(())
    }

    #[test]
    fn test_parse_clause_number() -> Result<(), Error> {
        let clause = b"v 131\n";

        let (_, clause_number) = _parse_clause_number(clause)?;
        assert_eq!(131, clause_number);

        let clause = b"v 0\n";

        let (_, clause_number) = _parse_clause_number(clause)?;
        assert_eq!(0, clause_number);

        Ok(())
    }

    #[test]
    fn test_parse_mus() -> Result<(), Error> {
        let mus = b"c [picomus] WARNING: no output file given
s UNSATISFIABLE
c [picomus] computed MUS of size 17 out of 814 (2%)
v 20
v 36
v 80
v 96
v 156
v 158
v 168
v 170
v 650
v 652
v 669
v 671
v 680
v 700
v 707
v 725
v 734
v 0
";

        let clauses = _parse_mus(mus)?;
        let expected_clauses = vec![
            20, 36, 80, 96, 156, 158, 168, 170, 650, 652, 669, 671, 680, 700, 707, 725, 734,
        ];

        assert_eq!(expected_clauses, clauses);
        Ok(())
    }
}
