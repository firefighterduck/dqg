use crate::{
    graph::VertexIndex,
    parser::{BinInput, BinParseResult},
    permutation::Permutation,
    Error,
};

use nom::{
    character::complete::{char, i32, line_ending, multispace0},
    combinator::map,
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, tuple},
};

fn parse_cycle(input: BinInput<'_>) -> BinParseResult<'_, Vec<VertexIndex>> {
    let cycle = separated_list1(char(','), preceded(multispace0, map(i32, |i| i - 1)));
    preceded(multispace0, delimited(char('('), cycle, char(')')))(input)
}

fn parse_permutation(input: BinInput<'_>, size: usize) -> BinParseResult<'_, Permutation> {
    map(many1(parse_cycle), |cycles| {
        Permutation::from_cycles(cycles, size)
    })(input)
}

fn parse_generators(input: BinInput<'_>, size: usize) -> BinParseResult<'_, Vec<Permutation>> {
    let generators = separated_list1(tuple((char(','), multispace0)), |input| {
        parse_permutation(input, size)
    });
    delimited(
        tuple((char('['), multispace0)),
        generators,
        tuple((multispace0, char(']'))),
    )(input)
}

pub fn parse_representatives(
    input: BinInput<'_>,
    size: usize,
) -> Result<Vec<Vec<Permutation>>, Error> {
    separated_list1(line_ending, |input| parse_generators(input, size))(input)
        .map(|(_, representatives)| representatives)
        .map_err(Error::from)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_cycle() -> Result<(), Error> {
        let cycle = "(1, 11,        13)".as_bytes();

        let expected = vec![0, 10, 12];
        let (_, parsed) = parse_cycle(cycle)?;
        assert_eq!(expected, parsed);

        Ok(())
    }

    #[test]
    fn test_parse_permutation() -> Result<(), Error> {
        let permutation = "(1,2,3,     4) 
(   23,    34,5)"
            .as_bytes();
        let cycles = vec![vec![0, 1, 2, 3], vec![22, 33, 4]];
        let size = 48;

        let expected = Permutation::from_cycles(cycles, size);
        let (_, parsed) = parse_permutation(permutation, size)?;
        assert_eq!(expected, parsed);

        Ok(())
    }

    #[test]
    fn test_parse_permutations() -> Result<(), Error> {
        let permutations = "[ (   66,   46, 54,2)(12,23),
(67,21,567, 65)
 ]"
        .as_bytes();
        let size = 1000;
        let cycles1 = vec![vec![65, 45, 53, 1], vec![11, 22]];
        let permutation1 = Permutation::from_cycles(cycles1, size);
        let cycles2 = vec![vec![66, 20, 566, 64]];
        let permutation2 = Permutation::from_cycles(cycles2, size);

        let expected = vec![permutation1, permutation2];
        let (_, parsed) = parse_generators(permutations, size)?;
        assert_eq!(expected, parsed);

        Ok(())
    }

    #[test]
    fn test_parse_representatives() -> Result<(), Error> {
        let reps = "[ (  1, 17)(  2, 18)(  3, 19)(  4, 20)(  5, 21), 
        (  9, 17, 25)( 10, 18, 26)( 11, 19, 27)( 12, 20, 28)]
[ (  1, 17)(  2, 18)(  3, 19) ]
"
        .as_bytes();
        let size = 30;
        let cycles1 = vec![
            vec![0, 16],
            vec![1, 17],
            vec![2, 18],
            vec![3, 19],
            vec![4, 20],
        ];
        let permutation1 = Permutation::from_cycles(cycles1, size);
        let cycles2 = vec![
            vec![8, 16, 24],
            vec![9, 17, 25],
            vec![10, 18, 26],
            vec![11, 19, 27],
        ];
        let permutation2 = Permutation::from_cycles(cycles2, size);
        let repr1 = vec![permutation1, permutation2];
        let cycles3 = vec![vec![0, 16], vec![1, 17], vec![2, 18]];
        let permutation3 = Permutation::from_cycles(cycles3, size);
        let repr2 = vec![permutation3];

        let expected = vec![repr1, repr2];
        let parsed = parse_representatives(reps, size)?;
        assert_eq!(expected, parsed);

        Ok(())
    }
}
