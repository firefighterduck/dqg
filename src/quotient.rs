//! Functionalities to build quotient graphs from
//! a set of generators and manage the orbits.

use custom_debug_derive::Debug;
use itertools::{Either, Itertools, MinMaxResult};
use libffi::high::{ClosureMut2, ClosureMut3, ClosureMut6};
use nauty_Traces_sys::{
    allgroup, densenauty, groupautomproc, grouplevelproc, groupptr, makecosetreps, optionblk,
    orbjoin, sparsenauty, statsblk, Traces, TracesStats, FALSE, TRUE,
};
use std::{os::raw::c_int, slice::from_raw_parts, usize};

use crate::{
    debug::print_generator,
    encoding::QuotientGraphEncoding,
    graph::{Graph, NautyGraph, SparseNautyGraph, TracesGraph, Vertex, VertexIndex, DEFAULT_COLOR},
    permutation::Permutation,
    Error, NautyTraces, Settings,
};

pub type Orbits = Vec<VertexIndex>;

/// Call nauty with the given graph representation
/// and compute the generators of the automorphism group
/// for the graph. Return the generators.
pub fn compute_generators_with_nauty(
    nauty_graph: Either<NautyGraph, SparseNautyGraph>,
    settings: &Settings,
) -> Vec<Permutation> {
    let mut generators = Vec::new();
    let (n, m);
    let mut options;

    match nauty_graph {
        Either::Left(ref dense_nauty_graph) => {
            let nm = dense_nauty_graph.graph_repr_sizes();
            n = nm.0;
            m = nm.1;
            options = optionblk::default();
        }
        Either::Right(ref sparse_nauty_graph) => {
            n = sparse_nauty_graph.partition.len();
            m = 0;
            options = optionblk::default_sparse();
        }
    }

    options.schreier = TRUE;

    if settings.colored_graph {
        options.defaultptn = FALSE;
    }

    let mut stats = statsblk::default();
    let mut orbits = vec![0_i32; n];

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

                generators.push(Permutation::new(generator));
            };
        let userautomproc = ClosureMut6::new(&mut userautomproc);

        options.userautomproc = Some(*userautomproc.code_ptr());

        // Safety: Call to nauty library function that computes
        // the automorphism group generator through useratomproc.
        match nauty_graph {
            Either::Left(mut dense_nauty_graph) => unsafe {
                densenauty(
                    dense_nauty_graph.adjacency_matrix.as_mut_ptr(),
                    dense_nauty_graph.vertex_order.as_mut_ptr(),
                    dense_nauty_graph.partition.as_mut_ptr(),
                    orbits.as_mut_ptr(),
                    &mut options,
                    &mut stats,
                    m as c_int,
                    n as c_int,
                    std::ptr::null_mut(),
                );
            },
            Either::Right(mut sparse_nauty_graph) => unsafe {
                sparsenauty(
                    &mut (&mut sparse_nauty_graph.sparse_graph).into(),
                    sparse_nauty_graph.vertex_order.as_mut_ptr(),
                    sparse_nauty_graph.partition.as_mut_ptr(),
                    orbits.as_mut_ptr(),
                    &mut options,
                    &mut stats,
                    std::ptr::null_mut(),
                );
            },
        }
    }

    generators
}

/// Call Traces with the given graph representation
/// and compute the generators of the automorphism group
/// for the graph. Return the generators.
pub fn compute_generators_with_traces(
    mut traces_graph: TracesGraph,
    settings: &Settings,
) -> Vec<Permutation> {
    let n = traces_graph.vertex_order.len();
    let mut generators = Vec::new();

    // Limit how long the closure can reference generators so that we can return it afterwards.
    {
        // Callback that copies the current generator.
        let mut userautomproc = |_count, generator_ptr: *mut c_int, n: c_int| {
            let mut generator = Vec::with_capacity(n as usize);
            let generator_raw = unsafe { from_raw_parts(generator_ptr, n as usize) };

            for vertex in generator_raw {
                generator.push(*vertex);
            }

            generators.push(Permutation::new(generator));
        };
        let userautomproc = ClosureMut3::new(&mut userautomproc);

        let mut options = nauty_Traces_sys::TracesOptions {
            userautomproc: Some(*userautomproc.code_ptr()),
            ..Default::default()
        };
        if settings.colored_graph {
            options.defaultptn = FALSE;
        }

        let mut stats = TracesStats::default();
        let mut orbits = vec![0_i32; n];

        // Safety: Call to Traces library function that computes
        // the automorphism group generators through useratomproc.
        unsafe {
            Traces(
                &mut (&mut traces_graph.sparse_graph).into(),
                traces_graph.vertex_order.as_mut_ptr(),
                traces_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                std::ptr::null_mut(),
            );
        }
    }

    generators
}

pub fn compute_generators(graph: &mut Graph, settings: &Settings) -> Vec<Permutation> {
    match settings.nauyt_or_traces {
        NautyTraces::Nauty => {
            let nauty_graph = NautyGraph::from_graph(graph);

            debug_assert!(nauty_graph.check_valid());
            compute_generators_with_nauty(Either::Left(nauty_graph), settings)
        }
        NautyTraces::SparseNauty => {
            let sparse_nauty_graph = SparseNautyGraph::from_graph(graph);
            compute_generators_with_nauty(Either::Right(sparse_nauty_graph), settings)
        }
        NautyTraces::Traces => {
            let traces_graph = TracesGraph::from_graph(graph);
            compute_generators_with_traces(traces_graph, settings)
        }
    }
}

#[cfg(not(tarpaulin_include))]
pub fn search_group(graph: &mut Graph, mut nauty_graph: NautyGraph, settings: &Settings) {
    let generators = compute_generators_with_nauty(Either::Left(nauty_graph.clone()), settings);

    for generator in generators {
        print!("Generator: ");
        print_generator(generator);
    }

    // First, call nauty to compute the group.
    let (n, m) = nauty_graph.graph_repr_sizes();
    let mut options = optionblk::default();

    if settings.colored_graph {
        options.defaultptn = FALSE;
    }

    let mut stats = statsblk::default();
    let mut orbits = vec![0_i32; n];

    // Set custom group generating methods.
    options.userautomproc = Some(groupautomproc);
    options.userlevelproc = Some(grouplevelproc);

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

    // Don't forget to sort. Otherwise, the encoding will be wrong.
    graph.sort();

    // Then search in the group.
    let mut handle_automorphism = |autom_ptr: *mut c_int, n: c_int| {
        let mut automorphism = Vec::with_capacity(n as usize);
        let automorphism_raw = unsafe { from_raw_parts(autom_ptr, n as usize) };

        for vertex in automorphism_raw {
            automorphism.push(*vertex);
        }

        let quotient = QuotientGraph::from_automorphism(graph, &mut automorphism);
        let formula = crate::encoding::encode_problem(&quotient, graph);

        if let Some((formula, _)) = formula {
            let descriptive = crate::sat_solving::solve(formula);

            if let Ok(true) = descriptive {
                print!("Descriptive induced by ");
                print_generator(Permutation::new_with_cycles(automorphism));
            } else {
                print!("Nondescriptive induced by ");
                print_generator(Permutation::new_with_cycles(automorphism));
            }
        } else {
            print!("Automorphism induced trivially descriptive: ");
            print_generator(Permutation::new_with_cycles(automorphism));
        }
    };
    let handle_automorphism = ClosureMut2::new(&mut handle_automorphism);

    unsafe {
        let group = groupptr(TRUE);
        if group.is_null() {
            panic!("The group ptr is null!");
        }
        makecosetreps(group);
        allgroup(group, Some(*handle_automorphism.code_ptr()));
    }
}

// Apply a generator to the current orbits and combine those,
// the the generator connects. Does not change the generator
// (the &mut is for FFI reasons only, will not write into it).
fn apply_generator(generator: &mut [VertexIndex], orbits: &mut Orbits) {
    debug_assert_eq!(generator.len(), orbits.len());

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

pub fn empty_orbits(number_vertices: usize) -> Orbits {
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
pub fn generate_orbits(generators: &mut [Permutation]) -> Orbits {
    let number_of_vertices = generators
        .get(0)
        .expect("Empty subset can't be used to generate orbits")
        .len();
    let mut orbits = empty_orbits(number_of_vertices);

    for generator in generators {
        apply_generator(&mut generator.raw, &mut orbits);
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
    #[cfg(not(tarpaulin_include))]
    fn from_automorphism(graph: &Graph, automorphism: &mut [VertexIndex]) -> Self {
        let mut orbits = empty_orbits(graph.size());
        apply_generator(automorphism, &mut orbits);
        Self::from_graph_orbits(graph, orbits)
    }

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
            quotient_graph = Graph::new_with_indices(&unique_orbits, true);
            // Add edges between the orbits if single vertices in these are
            // connected by an edge. Doesn't add edges within the same orbit.
            graph.iterate_edges().for_each(|(start, end)| {
                let start_orbit = get_orbit(&orbits, start);
                let end_orbit = get_orbit(&orbits, end);
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
                .set_vertex(Vertex::new(0, DEFAULT_COLOR))
                .expect("Single vertex could not be added!");
        }

        QuotientGraph {
            quotient_graph,
            orbits,
        }
    }

    pub fn get_orbit_sizes(&self) -> (usize, usize) {
        let mut counter = vec![0usize; self.orbits.len()];
        self.orbits
            .iter()
            .for_each(|orbit| counter[*orbit as usize] += 1);
        match counter.iter().filter(|size| **size > 0).minmax() {
            MinMaxResult::NoElements => (0, 0),
            MinMaxResult::OneElement(m) => (*m, *m),
            MinMaxResult::MinMax(min, max) => (*min, *max),
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn search_non_descriptive_core(self, graph: &Graph) -> Option<QuotientGraphEncoding> {
        use crate::encoding::{
            EdgeEncoding, HighLevelEncoding, SATEncoding, SATEncodingDictionary,
        };
        use rayon::prelude::*;
        let QuotientGraphEncoding(quotient_edges, orbits) = self.encode_high();

        orbits
            .iter()
            .cloned()
            .combinations(4) // From observations it seemed that such cores are mostly of size 4.
            .par_bridge()
            .find_map_any(|orbit_subset| {
                let mut dict = SATEncodingDictionary::default();
                let edge_subset = quotient_edges
                    .iter()
                    .filter(|edge| {
                        let (start, end) = edge.get_edge();
                        orbit_subset.iter().any(|(orbit, _)| *orbit == start)
                            && orbit_subset.iter().any(|(orbit, _)| *orbit == end)
                    })
                    .copied()
                    .collect::<Vec<EdgeEncoding>>();

                let descriptive_constraint_encoding =
                    QuotientGraphEncoding(edge_subset.clone(), orbit_subset.clone())
                        .encode_sat(&mut dict, graph);

                let transversal_encoding = orbit_subset
                    .iter()
                    .flat_map(|orbit| orbit.encode_sat(&mut dict, graph));

                if !crate::solve(
                    transversal_encoding.chain(descriptive_constraint_encoding.into_iter()),
                )
                .unwrap()
                {
                    Some(QuotientGraphEncoding(edge_subset, orbit_subset))
                } else {
                    None
                }
            })
    }

    pub fn induced_subquotient(&self, orbit_subset: &[VertexIndex]) -> Result<Self, Error> {
        let mut sub_orbits = self.orbits.clone();
        sub_orbits.iter_mut().for_each(|orbit| {
            if orbit_subset.binary_search(orbit).is_err() {
                *orbit = -1;
            }
        });

        Ok(QuotientGraph {
            quotient_graph: self.quotient_graph.induce_subgraph(orbit_subset, true)?,
            orbits: sub_orbits,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{graph::GraphError, Error};

    #[test]
    fn test_from_graph_orbits() -> Result<(), Error> {
        let mut graph = Graph::new_ordered(8);
        graph.add_edge(0, 1)?;
        graph.add_edge(0, 3)?;
        graph.add_edge(0, 4)?;
        graph.add_edge(1, 2)?;
        graph.add_edge(1, 5)?;
        graph.add_edge(2, 3)?;
        graph.add_edge(2, 6)?;
        graph.add_edge(3, 7)?;
        graph.add_edge(4, 5)?;
        graph.add_edge(4, 7)?;
        graph.add_edge(5, 6)?;
        graph.add_edge(6, 7)?;

        let orbits = vec![0, 1, 2, 1, 4, 0, 1, 0];

        let quotient = QuotientGraph::from_graph_orbits(&graph, orbits.clone());
        assert_eq!(orbits, quotient.orbits);

        let mut expected_vert0 = Vertex::new(0, DEFAULT_COLOR);
        expected_vert0.add_edge(1);
        expected_vert0.add_edge(4);
        let mut expected_vert1 = Vertex::new(1, DEFAULT_COLOR);
        expected_vert1.add_edge(0);
        expected_vert1.add_edge(2);
        let mut expected_vert2 = Vertex::new(2, DEFAULT_COLOR);
        expected_vert2.add_edge(1);
        let mut expected_vert4 = Vertex::new(4, DEFAULT_COLOR);
        expected_vert4.add_edge(0);

        assert_eq!(4, quotient.quotient_graph.size());
        assert_eq!(expected_vert0, *quotient.quotient_graph.get_vertex(0)?);
        assert_eq!(expected_vert1, *quotient.quotient_graph.get_vertex(1)?);
        assert_eq!(expected_vert2, *quotient.quotient_graph.get_vertex(2)?);
        assert_eq!(expected_vert4, *quotient.quotient_graph.get_vertex(4)?);

        // Single orbit
        let graph = Graph::new_ordered(1);
        let orbits = vec![0];

        let quotient = QuotientGraph::from_graph_orbits(&graph, orbits.clone());
        assert_eq!(orbits, quotient.orbits);
        assert_eq!(
            Vertex::new(0, DEFAULT_COLOR),
            *quotient.quotient_graph.get_vertex(0)?
        );
        assert_eq!(1, quotient.quotient_graph.size());

        Ok(())
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
        let mut generators = vec![
            vec![5, 1, 2, 6, 4, 0, 3, 7].into(),
            vec![0, 3, 2, 1, 4, 7, 6, 5].into(),
        ];
        let orbits = generate_orbits(&mut generators);
        assert_eq!(orbits, vec![0, 1, 2, 1, 4, 0, 1, 0]);
    }

    #[test]
    fn test_compute_generators_with_dense_nauty() -> Result<(), GraphError> {
        let settings = Settings {
            colored_graph: true,
            ..Default::default()
        };

        let mut graph = Graph::new_ordered(8);
        graph.add_edge(0, 1)?;
        graph.add_edge(0, 3)?;
        graph.add_edge(0, 4)?;
        graph.add_edge(1, 2)?;
        graph.add_edge(1, 5)?;
        graph.add_edge(2, 3)?;
        graph.add_edge(2, 6)?;
        graph.add_edge(3, 7)?;
        graph.add_edge(4, 5)?;
        graph.add_edge(4, 7)?;
        graph.add_edge(5, 6)?;
        graph.add_edge(6, 7)?;

        let order = [2, 0, 1, 3, 4, 5, 6, 7];
        let colours = [2, 2, 1, 2, 2, 2, 2, 2];
        graph.set_colours(&colours)?;
        graph.order(&order)?;

        // Test dense nauty
        let nauty_graph = NautyGraph::from_graph(&mut graph);
        assert!(nauty_graph.check_valid());
        let expected_generators: Vec<Permutation> = vec![
            vec![5, 1, 2, 6, 4, 0, 3, 7].into(),
            vec![0, 3, 2, 1, 4, 7, 6, 5].into(),
        ];
        let generators = compute_generators_with_nauty(Either::Left(nauty_graph), &settings);
        assert_eq!(expected_generators, generators);

        // Test sparse nauty
        let sparse_nauty_graph = SparseNautyGraph::from_graph(&mut graph);
        let expected_generators: Vec<Permutation> = vec![
            vec![0, 3, 2, 1, 4, 7, 6, 5].into(),
            vec![5, 1, 2, 6, 4, 0, 3, 7].into(),
        ];
        let generators =
            compute_generators_with_nauty(Either::Right(sparse_nauty_graph), &settings);
        assert_eq!(expected_generators, generators);

        // Test traces
        let traces_graph = TracesGraph::from_graph(&mut graph);
        let expected_generators: Vec<Permutation> = vec![
            vec![7, 3, 2, 6, 4, 0, 1, 5].into(),
            vec![5, 1, 2, 6, 4, 0, 3, 7].into(),
        ];
        let generators = compute_generators_with_traces(traces_graph, &settings);

        assert_eq!(expected_generators, generators);

        Ok(())
    }
}
