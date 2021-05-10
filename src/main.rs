#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::Itertools;
use std::{env, io};

mod graph;
use graph::{GraphError, VertexIndex};

mod input;
use input::{read_graph, read_vertex};

// mod combinatoric; unused for now, may be helpful for increasing performance lateron

mod quotient;
use quotient::{compute_generators_with_nauty, generate_orbits, QuotientGraph};

mod encoding;
use encoding::encode_problem;

mod sat_solving;
use sat_solving::solve;

mod parser;
use parser::ParseError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Graph initialisation error")]
    GraphError(GraphError),
    #[error("Error while parsing input file with graph description")]
    ParseError,
}

impl<'a> From<GraphError> for Error {
    fn from(ge: GraphError) -> Self {
        Self::GraphError(ge)
    }
}

impl<'a> From<nom::Err<ParseError<'a>>> for Error {
    fn from(_: nom::Err<ParseError<'a>>) -> Self {
        Self::ParseError
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph;

    // Either read from a file ...
    if env::args().len() > 1 {
        let input_path = env::args().nth(1).expect("usage: dqg FILE");
        let file = std::fs::read_to_string(&input_path)?;
        graph = parser::parse_dreadnaut_input(&file)?;
    } else {
        let stdin = io::stdin();

        // or initialize the graph for a number of vertices from stdin ...
        graph = read_graph(&stdin)?;

        // ..., then read and insert the edges and ...
        for i in 0..graph.size() {
            if !read_vertex(i as VertexIndex, &mut graph, &stdin)? {
                break;
            }
        }
    }

    // ... compute the generators with nauty. Then ...
    let nauty_graph = graph.prepare_nauty();
    assert!(nauty_graph.check_valid());
    let generators = compute_generators_with_nauty(nauty_graph);

    let f = |mut subset| {
        let orbits = generate_orbits(&mut subset);
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);
        let formula = encode_problem(&graph, &quotient_graph);
        println!("Quotient is descriptive?: {}", solve(formula));
    };

    // ... iterate over all possible subsets of generators.
    generators.into_iter().powerset().skip(1).for_each(f);

    Ok(())
}
