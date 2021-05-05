//! Functionalities to build quotient graphs from
//! a set of generators and manage the orbits.

use custom_debug_derive::Debug;
use itertools::Itertools;
use libffi::high::ClosureMut6;
use nauty_Traces_sys::{densenauty, optionblk, orbjoin, statsblk};
use std::{os::raw::c_int, slice::from_raw_parts};

use crate::graph::{Graph, NautyGraph, Vertex, VertexIndex};

pub type Generators = Vec<Vec<VertexIndex>>;
pub type Orbits = Vec<VertexIndex>;

/// Call nauty with the given graph representation
/// and compute the generators of the automorphism group
/// for the graph. Return the generators.
pub fn compute_generators_with_nauty(mut nauty_graph: NautyGraph) -> Generators {
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

        let mut options = optionblk::default();
        options.userautomproc = Some(*userautomproc.code_ptr());
        let mut stats = statsblk::default();
        let mut orbits = vec![0 as c_int; n];

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

fn get_orbit(orbits: &Orbits, vertex: VertexIndex) -> VertexIndex {
    *orbits
        .get(vertex as usize)
        .expect("Vertex not part of given orbits!")
}

// Generate the orbits of a quotient graph from the generators of the original graph.
pub fn generate_orbits(generators: &mut Generators) -> Orbits {
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
        let number_of_orbits = orbits.iter().unique().count();
        let mut quotient_graph = Graph::new_empty(number_of_orbits);

        // We don't need to search for edges if there can't be any.
        if number_of_orbits > 0 {
            // Add edges between the orbits if single vertices in these are
            // connected by and edge. Doesn't add edges within the same orbit.
            graph.iterate_edges(|(start, end)| {
                let start_orbit = get_orbit(&orbits, start);
                let end_orbit = get_orbit(&orbits, end);
                if start_orbit != end_orbit {
                    quotient_graph.add_arc(start_orbit, end_orbit);
                }
            });

            // Edges between orbits might be generated more often than once.
            quotient_graph.minimize();
        } else {
            quotient_graph.add_vertex(Vertex::new(0, -1));
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