//! Parser for graphs in a specific format.
//! The supported format is based of data from
//! https://snap.stanford.edu/data/ .

use std::io::BufRead;

use crate::{
    get_line_parse, get_line_recognize,
    graph::{Graph, VertexIndex},
    parse_single_line, Error,
};

use super::{Input, ParseResult};

fn parse_size_comment(input: Input<'_>) -> ParseResult<'_, usize> {
    use nom::{
        bytes::complete::tag,
        character::complete::{char, u64},
        combinator::map,
        sequence::{preceded, terminated, tuple},
    };

    let size_parser = preceded(tag(" Nodes: "), u64);
    let edges_parser = tuple((tag(" Edges: "), u64));
    let comment_parser = preceded(char('#'), terminated(size_parser, edges_parser));

    map(comment_parser, |size| size as usize)(input)
}

fn parse_meaningless_comment(input: Input<'_>) -> ParseResult<'_, ()> {
    use nom::{
        character::complete::{char, not_line_ending},
        combinator::value,
        sequence::tuple,
    };

    let comment_line_parser = tuple((char('#'), not_line_ending));
    value((), comment_line_parser)(input)
}

fn parse_edge(input: Input<'_>) -> ParseResult<'_, (VertexIndex, VertexIndex)> {
    use nom::{
        character::complete::{i32, multispace1},
        sequence::{pair, terminated},
    };

    pair(terminated(i32, multispace1), i32)(input)
}

pub fn parse_txt_input<B: BufRead>(input: B) -> Result<Graph, Error> {
    use nom::combinator::eof;

    let mut lines = input.lines().peekable();

    get_line_recognize!(lines, parse_meaningless_comment);
    get_line_recognize!(lines, parse_meaningless_comment);
    get_line_parse!(lines, graph_size, parse_size_comment);
    get_line_recognize!(lines, parse_meaningless_comment);

    let mut graph = Graph::new_ordered(graph_size);

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
    fn test_parse_size_comment() -> Result<(), Error> {
        let comment = "# Nodes: 18772 Edges: 396160\n";
        let (_, parsed) = parse_size_comment(comment)?;
        assert_eq!(18772, parsed);

        Ok(())
    }

    #[test]
    fn test_parse_meaningless_comment() -> Result<(), Error> {
        let comment = "# Directed graph (each unordered pair of nodes is saved once):\n";
        Ok(parse_meaningless_comment(comment)?.1)
    }

    #[test]
    fn test_parse_txt_input() -> Result<(), Error> {
        let txt = "# Directed graph (each unordered pair of nodes is saved once): CA-AstroPh.txt 
# Collaboration network of Arxiv Astro Physics category (there is an edge if authors coauthored at least one paper)
# Nodes: 6 Edges: 396160
# FromNodeId	ToNodeId
0	1
2	3
1	4
2	5
";
        let buf = BufReader::new(txt.as_bytes());
        let mut graph = Graph::new_ordered(6);
        graph.add_edge(0, 1)?;
        graph.add_edge(2, 3)?;
        graph.add_edge(1, 4)?;
        graph.add_edge(2, 5)?;

        let parsed = parse_txt_input(buf)?;

        assert_eq!(graph, parsed);

        Ok(())
    }
}
