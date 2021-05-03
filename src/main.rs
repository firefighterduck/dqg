#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use std::{
    error::Error,
    io::{self},
};

mod graph;
use graph::VertexIndex;

mod input;
use input::{read_graph, read_vertex};

mod combinatoric;
use combinatoric::iterate_powerset;

mod encoding;

mod quotient;
use quotient::{compute_generators_with_nauty, generate_orbits};

use crate::quotient::QuotientGraph;

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
    let mut generators = compute_generators_with_nauty(nauty_graph);

    let f = |subset: &mut Vec<Vec<i32>>| {
        let orbits = generate_orbits(subset);
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);
        println!(
            "Quotient for generator {:?}:\n {:?}",
            subset, quotient_graph
        );
    };

    // ... iterate over all possible subsets of generators.
    iterate_powerset(&mut generators, f);

    Ok(())
}
