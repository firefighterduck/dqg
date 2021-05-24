//! Parser for graph files in dreadnaut syntax.
//! E.g., these can be generated from planning
//! problems by this tool: https://home.in.tum.de/~mansour/cv-and-website/tools/quotientPlan.zip

use crate::{
    graph::{Colour, Graph, VertexIndex, DEFAULT_COLOR},
    Error,
};

pub type Input<'a> = &'a str;
pub type ParseError<'a> = nom::error::VerboseError<Input<'a>>;
pub type ParseResult<'a, O> = nom::IResult<Input<'a>, O, ParseError<'a>>;

/// The used graph generation tool writes always this header first.
/// This only encodes that Traces should print out information which
/// this tool doesn't need due to it using nauty/Traces as a library.
fn parse_header(input: Input<'_>) -> ParseResult<'_, ()> {
    use nom::{
        bytes::complete::tag, character::complete::line_ending, error::context, sequence::tuple,
    };

    let mut header = context(
        "Header from stripsSymmetry",
        tuple((
            tag("At"),
            line_ending,
            line_ending,
            tag("-a"),
            line_ending,
            tag("-m"),
            line_ending,
        )),
    );

    let (rest, _) = header(input)?;
    Ok((rest, ()))
}

/// Parse the start line for th graph that contains the size.
fn parse_graph_size(input: Input<'_>) -> ParseResult<'_, usize> {
    use nom::{
        bytes::complete::tag,
        character::complete::{digit1, line_ending},
        error::context,
        sequence::tuple,
    };

    let mut size_header = context(
        "Graph size header",
        tuple((tag("n="), digit1, tag(" g"), line_ending)),
    );
    let (rest, (_, graph_size, _, _)) = size_header(input)?;
    Ok((rest, graph_size.parse().unwrap()))
}

/// Parse a single vertex index.
fn parse_vertex_index(input: Input<'_>) -> ParseResult<'_, VertexIndex> {
    use nom::{character::complete::digit1, combinator::map};
    map(digit1, |index_str: &str| index_str.parse().unwrap())(input)
}

/// Parse the edges from vertex s from `s:e1 e2 e3 ... en`.
fn parse_vertex_edges(
    graph_size: usize,
    input: Input<'_>,
) -> ParseResult<'_, (VertexIndex, Vec<VertexIndex>)> {
    use nom::{
        bytes::complete::tag, character::complete::space1, combinator::verify, error::context,
        multi::separated_list1,
    };

    let (input, index) = context("lines starts with vector index", parse_vertex_index)(input)?;
    let (input, _) = tag(":")(input)?;

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

/// Parse the end of a egde line which determines
/// if the edge_lines stop early (`.`) or continues (`;`).
fn parse_continue_after_edge_line(input: Input<'_>) -> ParseResult<'_, bool> {
    use nom::{
        branch::alt,
        bytes::complete::tag,
        character::complete::{line_ending, space0},
        combinator::map,
        sequence::{preceded, terminated},
    };

    let edge_list_end = preceded(space0, alt((tag(";"), tag("."))));
    let edge_line_end = terminated(edge_list_end, line_ending);
    let mut should_continue_after_line = map(edge_line_end, |end: &str| end == ";");

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
        combinator::complete,
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
    let (rest, _) = complete(tuple((tag("] x o"), multispace1)))(input)?;

    for colour in colour_list {
        for vertex in colour {
            colours[vertex as usize] = colour_counter;
        }
        colour_counter += 1;
    }

    Ok((rest, colours))
}

pub fn parse_dreadnaut_input(input: Input<'_>) -> Result<Graph, Error> {
    let (input, _) = parse_header(input)?;
    let (mut input, graph_size) = parse_graph_size(input)?;
    let mut graph = Graph::new_ordered(graph_size);

    loop {
        let (rest, (vertex, edges)) = parse_vertex_edges(graph_size, input)?;

        for end in edges {
            graph.add_edge(vertex, end)?;
        }

        let (rest, should_continue) = parse_continue_after_edge_line(rest)?;
        input = rest;

        if !should_continue || vertex as usize >= graph_size - 1 {
            break;
        }
    }

    let (_, colours) = parse_colouring(graph_size, input)?;
    graph.set_colours(&colours)?;

    Ok(graph)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_header() {
        let test_input = "At\n\n-a\n-m\n";
        assert!(parse_header(test_input).is_ok());
    }

    #[test]
    fn test_parse_graph_size() {
        let test_size = 128;

        let valid_input = format!("n={} g\n", test_size);
        let (_, parsed_size) = parse_graph_size(&valid_input).unwrap();
        assert_eq!(test_size, parsed_size);

        let non_ternary_input = "n=0xfa g\n";
        assert!(parse_graph_size(non_ternary_input).is_err());

        let leading_zeros_input = format!("n=0000{} g\n", test_size);
        let (_, parsed_size) = parse_graph_size(&leading_zeros_input).unwrap();
        assert_eq!(test_size, parsed_size);
    }

    #[test]
    fn test_parse_vertex_index() {
        let test_index = 15632;

        let test_input = format!("{}", test_index);
        let (_, parsed_index) = parse_vertex_index(&test_input).unwrap();
        assert_eq!(test_index, parsed_index);

        let negative_input = format!("-{}", test_index);
        assert!(parse_vertex_index(&negative_input).is_err());
    }

    #[test]
    fn test_parse_vertex_edges() {
        let test_input = "12345:12 2 0 12 34235 88 23 ;";
        let test_size = i32::MAX;

        let (_, (vertex, edges)) = parse_vertex_edges(test_size as usize, test_input).unwrap();
        assert_eq!(12345, vertex);
        assert_eq!(vec![12, 2, 0, 12, 34235, 88, 23], edges);
    }

    #[test]
    fn test_parse_continue_after_edge_line() {
        let continue_input = "      ;\n";
        let (_, parsed_flag) = parse_continue_after_edge_line(continue_input).unwrap();
        assert!(parsed_flag);

        let quit_input = " .\n ";
        let (_, parsed_flag) = parse_continue_after_edge_line(quit_input).unwrap();
        assert!(!parsed_flag);
    }

    #[test]
    fn test_parse_colouring() {
        let test_input = "f=[1|  0  ,  3 | 2] x o\n\n";
        let (_, parsed_colours) = parse_colouring(5, test_input).unwrap();
        assert_eq!(vec![2, 1, 3, 2, DEFAULT_COLOR], parsed_colours);
    }

    #[test]
    fn test_parse_dreadnaut_input() {
        let test_file = r"At

-a
-m
n=4 g
0:1 2 ;
2:3;
3:0.
f=[0|1, 2] x o

        ";
        let mut expected_graph = Graph::new_ordered(4);
        expected_graph.add_edge(0, 1).unwrap();
        expected_graph.add_edge(0, 2).unwrap();
        expected_graph.add_edge(2, 3).unwrap();
        expected_graph.add_edge(3, 0).unwrap();
        expected_graph
            .set_colours(&vec![1, 2, 2, DEFAULT_COLOR])
            .unwrap();

        let parsed_graph = parse_dreadnaut_input(test_file).unwrap();
        assert_eq!(expected_graph, parsed_graph);
    }
}
