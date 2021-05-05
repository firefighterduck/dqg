#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::Itertools;
use std::{error::Error, io};

mod graph;
use graph::VertexIndex;

mod input;
use input::{read_graph, read_vertex};

// mod combinatoric; unused for now, may be helpful for increasing performance lateron

mod quotient;
use quotient::{compute_generators_with_nauty, generate_orbits, QuotientGraph};

mod encoding;
use encoding::encode_problem;

mod sat_solving;
use sat_solving::solve;

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();

    // Initialize the graph for a number of vertices ...
    let mut graph = read_graph(&stdin)?;

    // ..., then read and insert the edges and ...
    for i in 0..graph.size() {
        if !read_vertex(i as VertexIndex, &mut graph, &stdin)? {
            break;
        }
    }

    // ... compute the generators with nauty. Then ...
    let nauty_graph = graph.prepare_nauty();
    assert!(nauty_graph.check_valid());
    let generators = compute_generators_with_nauty(nauty_graph);

    let f = |mut subset| {
        let orbits = generate_orbits(&mut subset);
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);
        println!(
            "Quotient for generator {:?}:\n {:?}",
            subset, quotient_graph
        );
        let formula = encode_problem(&graph, &quotient_graph);
        println!("Quotient is descriptive?: {}", solve(formula));
    };

    // ... iterate over all possible subsets of generators.
    generators.into_iter().powerset().for_each(f);

    Ok(())
}
