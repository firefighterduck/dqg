//! Functionalities to build quotient graphs from
//! a set of generators and manage the orbits.

use custom_debug_derive::Debug;
use itertools::Itertools;
use libffi::high::ClosureMut6;
use nauty_Traces_sys::{densenauty, orbjoin, statsblk};
use std::{io::Write, os::raw::c_int, slice::from_raw_parts};

use crate::{
    encoding::HighLevelEncoding,
    graph::{Graph, NautyGraph, Vertex, VertexIndex},
};

pub type Generator = Vec<VertexIndex>;
pub type Orbits = Vec<VertexIndex>;

/// Call nauty with the given graph representation
/// and compute the generators of the automorphism group
/// for the graph. Return the generators.
#[cfg(not(tarpaulin_include))]
pub fn compute_generators_with_nauty(mut nauty_graph: NautyGraph) -> Vec<Generator> {
    let (n, m) = nauty_graph.graph_repr_sizes();
    let mut generators = Vec::new();

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

        let mut options = nauty_Traces_sys::optionstruct {
            userautomproc: Some(*userautomproc.code_ptr()),
            ..Default::default()
        };
        let mut stats = statsblk::default();
        let mut orbits = vec![0_i32; n];

        // Safety: Call to nauty library function that computes
        // the automorphism group generator through useratomproc.
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

// Apply a generator to the current orbits and combine those,
// the the generator connects. Does not change the generator
// (the &mut is for FFI reasons only, will not write into it).
fn apply_generator(generator: &mut [VertexIndex], orbits: &mut Orbits) {
    assert_eq!(generator.len(), orbits.len());

    // Safety: Call to nauty library function that reads from the generator
    // and combines orbits accordingly. There probably is no nicer way to do this.
    unsafe {
        orbjoin(
            orbits.as_mut_ptr(),
            generator.as_mut_ptr(),
            generator.len() as c_int,
        );
    }
}

fn empty_orbits(number_vertices: usize) -> Orbits {
    let mut orbits = Vec::with_capacity(number_vertices);

    for vertex in 0..number_vertices {
        orbits.push(vertex as VertexIndex);
    }

    orbits
}

fn get_orbit(orbits: &[VertexIndex], vertex: VertexIndex) -> VertexIndex {
    *orbits
        .get(vertex as usize)
        .expect("Vertex not part of given orbits!")
}

// Generate the orbits of a quotient graph from the generators of the original graph.
pub fn generate_orbits(generators: &mut [Generator]) -> Orbits {
    let number_of_vertices = generators
        .get(0)
        .expect("Empty subset can't be used to generate orbits")
        .len();
    let mut orbits = empty_orbits(number_of_vertices);

    for generator in generators {
        apply_generator(generator, &mut orbits);
    }

    orbits
}

#[cfg(not(tarpaulin_include))]
pub fn print_orbits_nauty_style(orbits: Orbits) {
    // This is necessary to give a correct
    // start point for the output.
    println!("cpu time = 0.00 seconds");

    orbits
        .encode_high(false)
        .into_iter()
        .for_each(|(orbit, members)| {
            if members.len() > 1 {
                members.iter().for_each(|member| print!("{} ", member));
                print!("({}); ", members.len());
            } else {
                print!("{}; ", orbit);
            }
        });

    // Force new line and flush everything out.
    println!();
    std::io::stdout()
        .flush()
        .expect("Why would stdout not be flushed?");
}

/// Represents a quotient graph where the vertices are
/// orbits. It also holds the reference to which original
/// vertices are part of which orbit.
#[derive(Debug)]
pub struct QuotientGraph {
    pub quotient_graph: Graph,
    #[debug(skip)]
    pub orbits: Orbits,
}

impl QuotientGraph {
    /// Generates the quotient graph where each orbit is represented
    /// by the vertex with the smallest index in the orbit.
    pub fn from_graph_orbits(graph: &Graph, orbits: Orbits) -> Self {
        let unique_orbits = orbits
            .iter()
            .unique()
            .copied()
            .collect::<Vec<VertexIndex>>();
        let mut quotient_graph;

        // We don't need to search for edges if there can't be any.
        if unique_orbits.len() > 1 {
            quotient_graph = Graph::new_with_indices(&unique_orbits);
            // Add edges between the orbits if single vertices in these are
            // connected by and edge. Doesn't add edges within the same orbit.
            graph.iterate_edges(|(start, end)| {
                let start_orbit = get_orbit(&orbits, *start);
                let end_orbit = get_orbit(&orbits, *end);
                if start_orbit != end_orbit {
                    quotient_graph
                        .add_arc(start_orbit, end_orbit)
                        .expect("Orbits not found in quotient graph!");
                }
            });

            // Edges between orbits might be generated more often than once.
            quotient_graph.minimize();
        } else {
            quotient_graph = Graph::new_ordered(1);
            quotient_graph
                .set_vertex(Vertex::new(0, -1))
                .expect("Single vertex could not be added!");
        }

        QuotientGraph {
            quotient_graph,
            orbits,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_graph_orbits() {
        let mut graph = Graph::new_ordered(8);
        assert_eq!(Ok(()), graph.add_edge(0, 1));
        assert_eq!(Ok(()), graph.add_edge(0, 3));
        assert_eq!(Ok(()), graph.add_edge(0, 4));
        assert_eq!(Ok(()), graph.add_edge(1, 2));
        assert_eq!(Ok(()), graph.add_edge(1, 5));
        assert_eq!(Ok(()), graph.add_edge(2, 3));
        assert_eq!(Ok(()), graph.add_edge(2, 6));
        assert_eq!(Ok(()), graph.add_edge(3, 7));
        assert_eq!(Ok(()), graph.add_edge(4, 5));
        assert_eq!(Ok(()), graph.add_edge(4, 7));
        assert_eq!(Ok(()), graph.add_edge(5, 6));
        assert_eq!(Ok(()), graph.add_edge(6, 7));

        let orbits = vec![0, 1, 2, 1, 4, 0, 1, 0];

        let mut quotient = QuotientGraph::from_graph_orbits(&graph, orbits.clone());
        assert_eq!(orbits, quotient.orbits);

        let mut expected_vert0 = Vertex::new(0, -1);
        expected_vert0.add_edge(1);
        expected_vert0.add_edge(4);
        let mut expected_vert1 = Vertex::new(1, -1);
        expected_vert1.add_edge(0);
        expected_vert1.add_edge(2);
        let mut expected_vert2 = Vertex::new(2, -1);
        expected_vert2.add_edge(1);
        let mut expected_vert4 = Vertex::new(4, -1);
        expected_vert4.add_edge(0);

        assert_eq!(4, quotient.quotient_graph.size());
        assert_eq!(
            expected_vert0,
            *quotient.quotient_graph.get_vertex(0).unwrap()
        );
        assert_eq!(
            expected_vert1,
            *quotient.quotient_graph.get_vertex(1).unwrap()
        );
        assert_eq!(
            expected_vert2,
            *quotient.quotient_graph.get_vertex(2).unwrap()
        );
        assert_eq!(
            expected_vert4,
            *quotient.quotient_graph.get_vertex(4).unwrap()
        );

        // Single orbit
        let graph = Graph::new_ordered(1);
        let orbits = vec![0];

        let mut quotient = QuotientGraph::from_graph_orbits(&graph, orbits.clone());
        assert_eq!(orbits, quotient.orbits);
        assert_eq!(
            Vertex::new(0, -1),
            *quotient.quotient_graph.get_vertex(0).unwrap()
        );
        assert_eq!(1, quotient.quotient_graph.size());
    }

    #[test]
    fn test_apply_generator() {
        let mut orbits = empty_orbits(7);
        let mut generator = [0, 1, 4, 3, 2, 6, 5];

        apply_generator(&mut generator, &mut orbits);

        assert_eq!(orbits, [0, 1, 2, 3, 2, 5, 5]);
    }

    #[test]
    fn test_generate_orbits() {
        let mut generators = vec![vec![5, 1, 2, 6, 4, 0, 3, 7], vec![0, 3, 2, 1, 4, 7, 6, 5]];
        let orbits = generate_orbits(&mut generators);
        assert_eq!(orbits, vec![0, 1, 2, 1, 4, 0, 1, 0]);
    }
}
