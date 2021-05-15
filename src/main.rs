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

#[cfg(feature = "statistics")]
use std::sync::Mutex;

mod graph;
use graph::{Graph, GraphError, VertexIndex};

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
use statistics::{QuotientStatistics, Statistics};

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
fn compute_quotient(
    generators_subset: &mut [Vec<VertexIndex>],
    #[cfg(feature = "statistics")] params: (Arc<Graph>, Arc<Mutex<Statistics>>),
    #[cfg(not(feature = "statistics"))] params: (Arc<Graph>, ()),
) {
    let orbits = generate_orbits(generators_subset);
    #[cfg(feature = "statistics")]
    let (min_orbit_size, max_orbit_size) = QuotientStatistics::log_orbit_sizes(&orbits);

    let quotient_graph = QuotientGraph::from_graph_orbits(&params.0, orbits);
    #[cfg(feature = "statistics")]
    let quotient_size = quotient_graph.quotient_graph.size();

    let formula = encode_problem(&params.0, &quotient_graph);
    #[cfg(feature = "statistics")]
    let formula_size = formula.len();
    let _descriptive = solve(formula);

    #[cfg(feature = "statistics")]
    if let Ok(mut statistics) = params.1.lock() {
        let quotient_stats = QuotientStatistics {
            max_orbit_size,
            min_orbit_size,
            quotient_size,
            formula_size,
            descriptive: _descriptive,
        };
        statistics.log_quotient_statistic(quotient_stats);
    };
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
    assert!(nauty_graph.check_valid());
    let generators = compute_generators_with_nauty(nauty_graph);
    #[cfg(feature = "statistics")]
    statistics.log_nauty_done();
    #[cfg(feature = "statistics")]
    statistics.log_number_of_generators(generators.len());

    #[cfg(feature = "statistics")]
    let statistics_arc = Arc::new(Mutex::new(statistics));
    let graph_arc = Arc::new(graph);
    let parameter_generator = || {
        (
            Arc::clone(&graph_arc),
            #[cfg(feature = "statistics")]
            Arc::clone(&statistics_arc),
            #[cfg(not(feature = "statistics"))]
            (),
        )
    };

    // ... iterate over all possible subsets of generators.
    iterate_powerset(generators, compute_quotient, parameter_generator);
    let mut statistics_file = File::create(statistics_path)?;

    #[cfg(feature = "statistics")]
    {
        let mut statistics = statistics_arc.lock().unwrap();
        statistics.log_end();

        write!(statistics_file, "{:#?}", statistics)?;
    }
    #[cfg(not(feature = "statistics"))]
    write!(statistics_file, "Statistics disabled!")?;

    Ok(())
}
