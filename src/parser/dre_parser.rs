//! Parser for graph files in dreadnaut syntax.
//! E.g., these can be generated from planning
//! problems by this tool: <https://home.in.tum.de/~mansour/cv-and-website/tools/quotientPlan.zip>

use std::{io::BufRead, iter::Peekable};

use nom::error::ParseError;

use crate::{
    get_line, get_line_parse,
    graph::{Colour, Graph, VertexIndex, DEFAULT_COLOR},
    parse_single_line, Error,
};

use super::{Input, ParseResult};

/// The used graph generation tool writes always this header first.
/// This only encodes that Traces should print out information which
/// this tool doesn't need due to it using nauty/Traces as a library.
fn parse_header<I>(input: &mut Peekable<I>) -> ParseResult<'_, ()>
where
    I: Iterator<Item = Result<String, std::io::Error>>,
{
    use nom::{
        bytes::complete::tag,
        error::{ErrorKind, VerboseError},
        Err,
    };

    let empty_peek = |line: &std::io::Result<String>| {
        let line = line.as_ref();
        if let Ok(line) = line {
            line == &"".to_string()
        } else {
            false
        }
    };
    let tag_peek = |tag_name| {
        move |line: &std::io::Result<String>| {
            let line = line.as_ref();
            if let Ok(line) = line {
                tag::<&str, &str, ()>(tag_name)(line).is_ok()
            } else {
                false
            }
        }
    };
    let empty_err = Err::Error(VerboseError::from_error_kind("", ErrorKind::CrLf));
    let tag_err = |tag| Err::Error(VerboseError::from_error_kind(tag, ErrorKind::Tag));

    let _ = input.next_if(tag_peek("At")).ok_or_else(|| tag_err("At"))?;
    let _ = input.next_if(empty_peek).ok_or(empty_err)?;
    let _ = input.next_if(tag_peek("-a")).ok_or_else(|| tag_err("-a"))?;
    let _ = input.next_if(tag_peek("-m")).ok_or_else(|| tag_err("-m"))?;

    Ok(("", ()))
}

/// Parse the start line for th graph that contains the size.
fn parse_graph_size(input: Input<'_>) -> ParseResult<'_, usize> {
    use nom::{bytes::complete::tag, character::complete::u64, error::context, sequence::tuple};

    let mut size_header = context("Graph size header", tuple((tag("n="), u64, tag(" g"))));
    let (rest, (_, graph_size, _)) = size_header(input)?;
    Ok((rest, graph_size as usize))
}

/// Parse a single vertex index.
fn parse_vertex_index(input: Input<'_>) -> ParseResult<'_, VertexIndex> {
    use nom::character::complete::i32;
    i32(input)
}

/// Parse the edges from vertex s from `s:e1 e2 e3 ... en`.
fn parse_vertex_edges(
    graph_size: usize,
    input: Input<'_>,
) -> ParseResult<'_, (VertexIndex, Vec<VertexIndex>)> {
    use nom::{
        bytes::complete::tag,
        character::complete::{space0, space1},
        combinator::verify,
        error::context,
        multi::separated_list1,
        sequence::pair,
    };

    let (input, index) = context("lines starts with vector index", parse_vertex_index)(input)?;
    let (input, _) = pair(tag(":"), space0)(input)?;

    let (rest, edges) = context(
        "List of edges from this vertex",
        separated_list1(
            space1,
            verify(parse_vertex_index, |end_index| {
                *end_index < graph_size as VertexIndex && *end_index != index
            }),
        ),
    )(input)?;

    Ok((rest, (index, edges)))
}

/// Parse the end of a edge line which determines
/// if the edge_lines stop early (`.`) or continues (`;`).
fn parse_continue_after_edge_line(input: Input<'_>) -> ParseResult<'_, bool> {
    use nom::{
        branch::alt, bytes::complete::tag, character::complete::space0, combinator::map,
        sequence::preceded,
    };

    let edge_list_end = preceded(space0, alt((tag(";"), tag("."))));
    let mut should_continue_after_line = map(edge_list_end, |end: &str| end == ";");

    should_continue_after_line(input)
}

/// Parse the colouring (i.e. the partition of the vertices). The input looks like this:
/// `f=[c11,c12.c13,...c1n|c21,c22,...c2m|...|cp1,cp2,...,cpk]`
/// Not specified vertices stay in colour DEFAULT_COLOR.
/// Also checks, that there is nothing of relevance after the colouring.
fn parse_colouring(graph_size: usize, input: Input<'_>) -> ParseResult<'_, Vec<Colour>> {
    use nom::{
        bytes::complete::tag,
        character::complete::{multispace1, space0},
        combinator::opt,
        multi::{separated_list0, separated_list1},
        sequence::tuple,
    };

    let mut colours = vec![DEFAULT_COLOR; graph_size];
    let mut colour_counter = 1;

    let sep = |sep_tag| tuple((space0, tag(sep_tag), space0));

    let single_colour = separated_list1(sep(","), parse_vertex_index);
    let mut colour_list = separated_list0(sep("|"), single_colour);

    let (input, _) = tag("f=[")(input)?;
    let (input, colour_list) = colour_list(input)?;
    let (rest, _) = tuple((tag("]"), opt(tag(" x o")), opt(multispace1)))(input)?;

    for colour in colour_list {
        for vertex in colour {
            colours[vertex as usize] = colour_counter;
        }
        colour_counter += 1;
    }

    Ok((rest, colours))
}

pub fn parse_dreadnaut_input<B: BufRead>(input: B) -> Result<(Graph, bool), Error> {
    use nom::combinator::eof;

    let mut lines = input.lines().peekable();

    let header = parse_header(&mut lines).is_ok();
    get_line_parse!(lines, graph_size, parse_graph_size);

    let mut graph = Graph::new_ordered(graph_size);

    loop {
        get_line!(line, lines);
        let (res, vertex_edges) = parse_vertex_edges(graph_size, &line)?;
        let (vertex, edges) = vertex_edges;

        for end in edges {
            graph.add_edge(vertex, end)?;
        }

        parse_single_line!(should_continue, parse_continue_after_edge_line(res));

        if !should_continue || vertex as usize >= graph_size - 1 {
            break;
        }
    }

    get_line!(color_line, lines);
    parse_single_line!(colours, parse_colouring(graph_size, &color_line));
    graph.set_colours(&colours)?;

    Ok((graph, header))
}

#[cfg(test)]
mod test {
    use std::io::BufReader;

    use super::*;

    #[test]
    fn test_parse_header() -> Result<(), Error> {
        let test_input = "At

-a
-m
";
        let test_buf = BufReader::new(test_input.as_bytes());
        let mut test_lines = test_buf.lines().peekable();
        Ok(parse_header(&mut test_lines)?.1)
    }

    #[test]
    fn test_parse_graph_size() -> Result<(), Error> {
        let test_size = 128;

        let valid_input = format!("n={} g\n", test_size);
        let (_, parsed_size) = parse_graph_size(&valid_input)?;
        assert_eq!(test_size, parsed_size);

        let non_ternary_input = "n=0xfa g\n";
        assert!(parse_graph_size(non_ternary_input).is_err());

        let leading_zeros_input = format!("n=0000{} g\n", test_size);
        let (_, parsed_size) = parse_graph_size(&leading_zeros_input)?;
        assert_eq!(test_size, parsed_size);

        Ok(())
    }

    #[test]
    fn test_parse_vertex_index() -> Result<(), Error> {
        let test_index = 15632;

        let test_input = format!("{}", test_index);
        let (_, parsed_index) = parse_vertex_index(&test_input)?;
        assert_eq!(test_index, parsed_index);

        let negative_input = format!("-{}", test_index);
        let (_, negative_parsed_index) = parse_vertex_index(&negative_input)?;
        assert_eq!(-test_index, negative_parsed_index);

        Ok(())
    }

    #[test]
    fn test_parse_vertex_edges() -> Result<(), Error> {
        let test_input = "12345:12 2 0 12 34235 88 23 ;";
        let test_size = i32::MAX;

        let (_, (vertex, edges)) = parse_vertex_edges(test_size as usize, test_input)?;
        assert_eq!(12345, vertex);
        assert_eq!(vec![12, 2, 0, 12, 34235, 88, 23], edges);

        Ok(())
    }

    #[test]
    fn test_parse_continue_after_edge_line() -> Result<(), Error> {
        let continue_input = "      ;\n";
        let (_, parsed_flag) = parse_continue_after_edge_line(continue_input)?;
        assert!(parsed_flag);

        let quit_input = " .\n ";
        let (_, parsed_flag) = parse_continue_after_edge_line(quit_input)?;
        assert!(!parsed_flag);

        Ok(())
    }

    #[test]
    fn test_parse_colouring() -> Result<(), Error> {
        let test_input = "f=[1|  0  ,  3 | 2] x o\n\n";
        let (_, parsed_colours) = parse_colouring(5, test_input)?;
        assert_eq!(vec![2, 1, 3, 2, DEFAULT_COLOR], parsed_colours);

        Ok(())
    }

    #[test]
    fn test_parse_dreadnaut_input() -> Result<(), Error> {
        let test_file = "At

-a
-m
n=4 g
0:1 2 ;
2:3;
3:0.
f=[0|1, 2] x o

        ";
        let test_buf = BufReader::new(test_file.as_bytes());
        let mut expected_graph = Graph::new_ordered(4);
        expected_graph.add_edge(0, 1)?;
        expected_graph.add_edge(0, 2)?;
        expected_graph.add_edge(2, 3)?;
        expected_graph.add_edge(3, 0)?;
        expected_graph.set_colours(&vec![1, 2, 2, DEFAULT_COLOR])?;

        let (parsed_graph, has_header) = parse_dreadnaut_input(test_buf)?;
        assert_eq!(expected_graph, parsed_graph);
        assert!(has_header);

        Ok(())
    }

    #[test]
    fn test_parse_dreadnaut_input_wo_header() -> Result<(), Error> {
        let test_file = "n=4 g
0:1 2 ;
2:3;
3:0.
f=[0|1, 2] 

        ";
        let test_buf = BufReader::new(test_file.as_bytes());
        let mut expected_graph = Graph::new_ordered(4);
        expected_graph.add_edge(0, 1)?;
        expected_graph.add_edge(0, 2)?;
        expected_graph.add_edge(2, 3)?;
        expected_graph.add_edge(3, 0)?;
        expected_graph.set_colours(&vec![1, 2, 2, DEFAULT_COLOR])?;

        let (parsed_graph, has_header) = parse_dreadnaut_input(test_buf)?;
        assert_eq!(expected_graph, parsed_graph);
        assert!(!has_header);

        Ok(())
    }
}
