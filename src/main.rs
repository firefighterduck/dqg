#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use std::{env, io, sync::Arc};

mod graph;
use graph::{GraphError, VertexIndex};

mod input;
use input::{read_graph, read_vertex};

mod combinatoric;
use combinatoric::iterate_powerset;

mod quotient;
use quotient::{compute_generators_with_nauty, generate_orbits, QuotientGraph};

mod encoding;
use encoding::encode_problem;

mod sat_solving;
use sat_solving::solve;

mod parser;
use parser::{parse_dreadnaut_input, ParseError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Graph initialisation error")]
    GraphError(GraphError),
    #[error("Error while parsing input file with graph description")]
    ParseError,
}

impl<'a> From<GraphError> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(ge: GraphError) -> Self {
        Self::GraphError(ge)
    }
}

impl<'a> From<nom::Err<ParseError<'a>>> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(_: nom::Err<ParseError<'a>>) -> Self {
        Self::ParseError
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph;

    // Either read from a file ...
    if env::args().len() > 1 {
        let input_path = env::args().nth(1).expect("usage: dqg FILE");
        let file = std::fs::read_to_string(&input_path)?;
        graph = parse_dreadnaut_input(&file)?;
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
    println!("{:?} generators", generators.len());

    let graph_arc = Arc::new(&graph);

    let f = |subset: &mut [Vec<VertexIndex>]| {
        //println!("For subset {:?}", subset);
        let orbits = generate_orbits(subset);
        //println!("For subset {:?} with orbits {:?}", subset, orbits);
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph_arc, orbits);
        let formula = encode_problem(&graph_arc, &quotient_graph);
        //println!("Resulting quotient graph {:?}", quotient_graph);
        if !solve(formula) {
            println!("Found a non-descriptive quotient!");
        }
    };

    // ... iterate over all possible subsets of generators.
    iterate_powerset(generators, f);

    Ok(())
}
