#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use std::{
    error::Error,
    io::{self},
    os::raw::c_int,
    slice::from_raw_parts,
    usize,
};

use libffi::high::ClosureMut6;
use nauty_Traces_sys::{densenauty, optionblk, statsblk};

mod graph;
use graph::{NautyGraph, VertexIndex};

mod input;
use input::{read_graph, read_vertex};

mod combinatoric;
use combinatoric::iterate_powerset;

mod encoding;

mod quotient;
use quotient::{generate_orbits, Generators};

/// Call nauty with the given graph representation
/// and compute the generators of the automorphism group
/// for the graph. Return the generators.
fn compute_generators_with_nauty(mut nauty_graph: NautyGraph) -> Generators {
    let (n, m) = nauty_graph.graph_repr_sizes();
    let mut generators = Vec::new();

    let mut options = optionblk::default();
    let mut stats = statsblk::default();
    let mut orbits = vec![0 as c_int; n];

    // Limit how long the closure can reference generators so that we can return it afterwards.
    {
        // Callback that copies the current generator.
        let mut userautomproc =
            |_count, generator_ptr: *mut c_int, _orbits, _numorbits, _stabvertex, n: c_int| {
                let mut generator = Vec::with_capacity(n as usize);
                let generator_raw = unsafe { from_raw_parts(generator_ptr, n as usize) };

                for vertex in generator_raw {
                    generator.push(*vertex);
                }

                generators.push(generator);
            };
        let userautomproc = ClosureMut6::new(&mut userautomproc);
        options.userautomproc = Some(*userautomproc.code_ptr());

        unsafe {
            densenauty(
                nauty_graph.adjacency_matrix.as_mut_ptr(),
                nauty_graph.vertex_order.as_mut_ptr(),
                nauty_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                m as c_int,
                n as c_int,
                std::ptr::null_mut(),
            );
        }
    }
    generators
}

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
        println!("Orbits for generator {:?}:\n {:?}", subset, orbits);
    };

    // ... iterate over all possible subsets of generators.
    iterate_powerset(&mut generators, f);

    Ok(())
}
