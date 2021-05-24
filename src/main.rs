#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::Itertools;
use std::time::Instant;

mod graph;
use graph::{Graph, VertexIndex};

mod input;
use input::read_graph;

mod quotient;
use quotient::{
    compute_generators_with_nauty, generate_orbits, print_orbits_nauty_style, QuotientGraph,
};

mod encoding;
use encoding::{
    encode_graph_edges, encode_problem, Formula, HighLevelEncoding, SATEncodingDictionary,
};

mod sat_solving;
use sat_solving::solve;

mod parser;

mod statistics;
use statistics::{OrbitStatistics, QuotientStatistics, Statistics};

mod debug;
pub use debug::Error;

#[cfg(not(tarpaulin_include))]
pub fn do_if_some<F, T>(optional: &mut Option<T>, f: F)
where
    F: FnOnce(&mut T),
{
    if let Some(val) = optional {
        f(val);
    }
}

pub struct Settings {
    /// Iterate the whole powerset.
    pub iter_powerset: bool,
    /// Compute only orbits.
    pub orbits_only: bool,
    /// Log orbit sizes.
    pub log_orbits: bool,
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient_with_statistics(
    generators_subset: &mut [Vec<VertexIndex>],
    graph: &Graph,
    graph_edges_encoding: Formula,
    sat_encoding_dict: &mut SATEncodingDictionary,
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
        for orbit in orbits.encode_high(false) {
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
        encode_problem(&quotient_graph, graph_edges_encoding, sat_encoding_dict)
    );

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
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient(
    generators_subset: &mut [Vec<VertexIndex>],
    graph: &Graph,
    graph_edges_encoding: Formula,
    sat_encoding_dict: &mut SATEncodingDictionary,
) {
    let orbits = generate_orbits(generators_subset);

    let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);

    let formula = encode_problem(&quotient_graph, graph_edges_encoding, sat_encoding_dict);
    let descriptive = solve(formula);

    if descriptive.is_ok() && !descriptive.unwrap() {
        eprintln!("Found a non descriptive quotient!");
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Error> {
    // Read the graph form a file or via CLI and ...
    let (mut graph, mut statistics, settings) = read_graph()?;

    // ... compute the generators with nauty. Then ...
    let nauty_graph = graph.prepare_nauty();
    assert!(nauty_graph.check_valid());
    let mut generators = compute_generators_with_nauty(nauty_graph);

    do_if_some(&mut statistics, Statistics::log_nauty_done);
    do_if_some(&mut statistics, |st| {
        st.log_number_of_generators(generators.len())
    });

    if settings.orbits_only {
        // TODO apply heuristic beforehand
        let orbits = generate_orbits(&mut generators);
        print_orbits_nauty_style(orbits);
        return Ok(());
    }

    // ... generate the shared encoding of the input graph and ...
    let mut sat_encoding_dict = SATEncodingDictionary::new();
    let graph_edges_encoding = encode_graph_edges(&graph, &mut sat_encoding_dict);

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
                        graph_edges_encoding.clone(),
                        &mut sat_encoding_dict,
                        &settings,
                        &mut statistics,
                    )
                });
        } else {
            compute_quotient_with_statistics(
                &mut generators,
                &graph,
                graph_edges_encoding,
                &mut sat_encoding_dict,
                &settings,
                &mut statistics,
            );
        }

        statistics.log_end();
        statistics.save_statistics()?;
    } else {
        // ... or without.
        {
            if settings.iter_powerset {
                generators
                    .into_iter()
                    .powerset()
                    .skip(1)
                    .for_each(|mut subset| {
                        compute_quotient(
                            &mut subset,
                            &graph,
                            graph_edges_encoding.clone(),
                            &mut sat_encoding_dict,
                        )
                    });
            } else {
                compute_quotient(
                    &mut generators,
                    &graph,
                    graph_edges_encoding,
                    &mut sat_encoding_dict,
                );
            }
        }
    }

    Ok(())
}
