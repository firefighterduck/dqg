#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::{Either, Itertools};
use std::time::Instant;

mod graph;
use graph::{Graph, VertexIndex};

mod input;
use input::read_graph;

mod quotient;
use quotient::{
    compute_generators_with_nauty, compute_generators_with_traces, generate_orbits, QuotientGraph,
};

mod encoding;
use encoding::{encode_problem, EdgeCache, HighLevelEncoding};

mod sat_solving;
use sat_solving::solve;

mod parser;

mod statistics;
use statistics::{OrbitStatistics, QuotientStatistics, Statistics};

mod debug;
pub use debug::Error;
use debug::{print_formula, print_orbits_nauty_style};

use crate::{
    encoding::cache_graph_edges,
    quotient::{empty_orbits, Orbits},
};

#[cfg(not(tarpaulin_include))]
pub fn do_if_some<F, T>(optional: &mut Option<T>, f: F)
where
    F: FnOnce(&mut T),
{
    if let Some(val) = optional {
        f(val);
    }
}

#[derive(Debug)]
pub enum NautyTraces {
    /// Calls dense nauty
    Nauty,
    /// Calls sparse nauty
    SparseNauty,
    /// Calls Traces (only for sparse graphs)
    Traces,
}

impl Default for NautyTraces {
    fn default() -> Self {
        Self::Nauty
    }
}

#[derive(Debug, Default)]
pub struct Settings {
    /// Iterate the whole powerset.
    pub iter_powerset: bool,
    /// Compute only orbits.
    pub orbits_only: bool,
    /// Log orbit sizes.
    pub log_orbits: bool,
    /// Print formula instead of solving it.
    pub print_formula: bool,
    /// Graph is colored and colors should be
    /// included in the nauty computation.
    pub colored_graph: bool,
    ///  Call nauty or traces.
    pub nauyt_or_traces: NautyTraces,
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient_with_statistics(
    generators_subset: &mut [Vec<VertexIndex>],
    graph: &Graph,
    edge_cache: &EdgeCache,
    settings: &Settings,
    statistics: &mut Statistics,
) {
    let start_time = Instant::now();

    time!(orbit_gen_time, orbits, generate_orbits(generators_subset));
    time!(
        _log_orbit_time,
        min_max_orbit_size,
        QuotientStatistics::log_orbit_sizes(&orbits)
    );
    let (min_orbit_size, max_orbit_size) = min_max_orbit_size;

    let mut orbit_sizes = OrbitStatistics::default();
    if settings.log_orbits {
        for orbit in orbits.encode_high() {
            orbit_sizes.log_orbit(&orbit);
        }
    }

    time!(
        quotient_gen_time,
        quotient_graph,
        QuotientGraph::from_graph_orbits(&graph, orbits)
    );
    let quotient_size = quotient_graph.quotient_graph.size();

    time!(
        encoding_time,
        formula,
        encode_problem(&quotient_graph, edge_cache)
    );

    if let Some(formula) = formula {
        if settings.print_formula {
            print_formula(formula);
            return;
        }

        time!(kissat_time, descriptive, solve(formula));

        let quotient_handling_time = start_time.elapsed();
        let quotient_stats = QuotientStatistics {
            quotient_size,
            max_orbit_size,
            min_orbit_size,
            descriptive,
            quotient_handling_time,
            kissat_time,
            orbit_gen_time,
            quotient_gen_time,
            encoding_time,
            orbit_sizes,
        };
        statistics.log_quotient_statistic(quotient_stats);
    } else {
        println!("Trivially descriptive");
    }
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient(
    generators_subset: &mut [Vec<VertexIndex>],
    graph: &Graph,
    edge_cache: &EdgeCache,
    settings: &Settings,
) {
    let orbits = generate_orbits(generators_subset);

    let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);

    let formula = encode_problem(&quotient_graph, edge_cache);

    if let Some(formula) = formula {
        if settings.print_formula {
            print_formula(formula);
            return;
        }

        let descriptive = solve(formula);

        if descriptive.is_ok() && !descriptive.unwrap() {
            eprintln!("Found a non descriptive quotient!");
        }
    } else {
        println!("Trivially descriptive");
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Error> {
    // Read the graph from a file or via CLI and ...
    let (mut graph, mut statistics, settings) = read_graph()?;
    let mut generators;

    // ... compute the generators with nauty or Traces. Then ...
    match settings.nauyt_or_traces {
        NautyTraces::Nauty => {
            let nauty_graph = graph.prepare_nauty();

            assert!(nauty_graph.check_valid());
            generators = compute_generators_with_nauty(Either::Left(nauty_graph), &settings);
        }
        NautyTraces::SparseNauty => {
            let sparse_nauty_graph = graph.prepare_sparse_nauty();
            generators =
                compute_generators_with_nauty(Either::Right(sparse_nauty_graph), &settings);
        }
        NautyTraces::Traces => {
            let traces_graph = graph.prepare_traces();
            generators = compute_generators_with_traces(traces_graph, &settings);
        }
    };

    do_if_some(&mut statistics, Statistics::log_nauty_done);
    do_if_some(&mut statistics, |st| {
        st.log_number_of_generators(generators.len())
    });

    if settings.orbits_only {
        // TODO apply heuristic beforehand
        let orbits: Orbits;

        if generators.is_empty() {
            orbits = empty_orbits(graph.size());
        } else {
            orbits = generate_orbits(&mut generators);
        }

        print_orbits_nauty_style(orbits);
        return Ok(());
    }

    let mut edge_cache = EdgeCache::new(graph.size());
    cache_graph_edges(&graph, &mut edge_cache);

    // ... iterate over the specified subsets of generators...
    if let Some(mut statistics) = statistics {
        // ... with statistics ...
        if settings.iter_powerset {
            generators
                .into_iter()
                .powerset()
                .skip(1)
                .for_each(|mut subset| {
                    compute_quotient_with_statistics(
                        &mut subset,
                        &graph,
                        &edge_cache,
                        &settings,
                        &mut statistics,
                    )
                });
        } else if !generators.is_empty() {
            compute_quotient_with_statistics(
                &mut generators,
                &graph,
                &edge_cache,
                &settings,
                &mut statistics,
            );
        }

        statistics.log_end();
        statistics.save_statistics()?;
    } else {
        // ... or without.
        if settings.iter_powerset {
            generators
                .into_iter()
                .powerset()
                .skip(1)
                .for_each(|mut subset| {
                    compute_quotient(&mut subset, &graph, &edge_cache, &settings)
                });
        } else if !generators.is_empty() {
            compute_quotient(&mut generators, &graph, &edge_cache, &settings);
        }
    }

    Ok(())
}
