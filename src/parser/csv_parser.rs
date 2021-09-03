//! Parser for graphs encoded in csv files.

use std::io::BufRead;

use crate::{
    get_line_recognize,
    graph::{Graph, VertexIndex},
    parse_single_line, Error,
};

use super::{Input, ParseResult};

fn parse_edge(input: Input<'_>) -> ParseResult<'_, (VertexIndex, VertexIndex)> {
    use nom::{
        character::complete::{char, i32},
        sequence::separated_pair,
    };

    separated_pair(i32, char(','), i32)(input)
}

fn parse_column_header(input: Input<'_>) -> ParseResult<'_, ()> {
    use nom::{character::complete::not_line_ending, combinator::value};

    value((), not_line_ending)(input)
}

pub fn parse_csv_input<B: BufRead>(graph_size: usize, input: B) -> Result<Graph, Error> {
    use nom::combinator::eof;

    let mut graph = Graph::new_ordered(graph_size);
    let mut lines = input.lines();

    get_line_recognize!(lines, parse_column_header);

    for line in lines {
        let line = line?;
        parse_single_line!(start_end, parse_edge(&line));
        let (start, end) = start_end;
        graph
            .add_edge(start, end)
            .expect("Edge to non existing vertex! Graph too small!");
    }

    Ok(graph)
}

#[cfg(test)]
mod test {
    use std::io::BufReader;

    use crate::Error;

    use super::*;

    #[test]
    fn test_parse_edge() -> Result<(), Error> {
        let edge = "123,46783";
        let parsed = parse_edge(edge)?.1;
        assert_eq!((123, 46783), parsed);

        Ok(())
    }

    #[test]
    fn test_parse_column_header() -> Result<(), Error> {
        let header = "node_1,node_2\n";
        Ok(parse_column_header(header)?.1)
    }

    #[test]
    fn test_parse_csv_input() -> Result<(), Error> {
        let csv = "node_1,node_2
0,3
0,1
0,6
1,10
1,3
";
        let buf = BufReader::new(csv.as_bytes());
        let parsed = parse_csv_input(11, buf)?;

        let mut graph = Graph::new_ordered(11);
        graph.add_edge(0, 3)?;
        graph.add_edge(0, 1)?;
        graph.add_edge(0, 6)?;
        graph.add_edge(1, 10)?;
        graph.add_edge(1, 3)?;

        assert_eq!(graph, parsed);

        Ok(())
    }
}
