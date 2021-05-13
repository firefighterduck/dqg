#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use std::{
    env::{self, current_dir},
    fs::{read_to_string, File},
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

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

#[cfg(feature = "statistics")]
mod statistics;
#[cfg(feature = "statistics")]
use statistics::Statistics;

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
    let mut statistics_path;

    // Either read from a file ...
    if env::args().len() > 1 {
        let input_path = env::args().nth(1).expect("usage: dqg FILE");
        let file = read_to_string(&input_path)?;

        statistics_path = PathBuf::from(&input_path);
        statistics_path.set_extension("dqg");

        graph = parse_dreadnaut_input(&file)?;
    } else {
        let stdin = io::stdin();

        statistics_path =
            current_dir().expect("Statistics feature requires current directory to be accessible!");
        statistics_path.push("statistics.dqg");

        // or initialize the graph for a number of vertices from stdin ...
        graph = read_graph(&stdin)?;

        // ..., then read and insert the edges and ...
        for i in 0..graph.size() {
            if !read_vertex(i as VertexIndex, &mut graph, &stdin)? {
                break;
            }
        }
    }

    #[cfg(feature = "statistics")]
    let mut statistics = Statistics::new(graph.size());

    // ... compute the generators with nauty. Then ...
    let nauty_graph = graph.prepare_nauty();
    #[cfg(feature = "statistics")]
    statistics.log_nauty_done();
    assert!(nauty_graph.check_valid());

    let generators = compute_generators_with_nauty(nauty_graph);
    #[cfg(feature = "statistics")]
    statistics.log_number_of_generators(generators.len());

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

    #[cfg(feature = "statistics")]
    statistics.log_end();

    let mut statistics_file = File::create(statistics_path)?;
    #[cfg(feature = "statistics")]
    write!(statistics_file, "{:#?}", statistics)?;
    #[cfg(not(feature = "statistics"))]
    write!(statistics_file, "Statistics disabled!")?;

    Ok(())
}
