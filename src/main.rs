#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::Itertools;
use std::{
    io::BufRead,
    time::{Duration, Instant},
};

mod graph;
use graph::{Graph, NautyGraph};

mod input;
use input::read_graph;

mod quotient;
use quotient::{compute_generators, generate_orbits, search_group, QuotientGraph};

mod encoding;
use encoding::{encode_problem, HighLevelEncoding};

mod sat_solving;
use sat_solving::{solve, solve_validate};

mod parser;

mod statistics;
use statistics::{OrbitStatistics, QuotientStatistics, Statistics};

mod debug;
pub use debug::Error;

mod permutation;
use permutation::Permutation;

mod metric;

mod transversal;
use transversal::is_transversal_consistent;

mod misc;
pub use misc::{do_if_some, MetricUsed, NautyTraces, Settings};

mod evaluate;
use evaluate::{evaluate_log_file, evaluate_logs};

mod gap;
use gap::gap_mode;

mod core;
use crate::core::search_with_core;

#[cfg(not(tarpaulin_include))]
fn compute_quotient_with_statistics(
    generators_subset: &mut [Permutation],
    graph: &Graph,
    settings: &mut Settings,
) -> bool {
    let start_time = Instant::now();

    time!(orbit_gen_time, orbits, generate_orbits(generators_subset));

    let mut orbit_sizes = OrbitStatistics::default();
    if settings.log_orbits {
        for orbit in orbits.encode_high() {
            orbit_sizes.log_orbit(&orbit);
        }
    }

    time!(
        quotient_gen_time,
        quotient_graph,
        QuotientGraph::from_graph_orbits(graph, orbits)
    );
    let quotient_size = quotient_graph.quotient_graph.size();
    let (min_orbit_size, max_orbit_size) = quotient_graph.get_orbit_sizes();

    time!(
        encoding_time,
        encoded,
        encode_problem(&quotient_graph, graph)
    );

    let mut descriptive = Ok(true);
    let mut validated = None;
    let mut kissat_time = Duration::ZERO;

    let return_val = if let Some((formula, dict)) = encoded {
        time!(k_time, descriptive_validated, {
            if settings.validate {
                let sat_result = solve_validate(formula, dict);
                match sat_result {
                    Ok(transversal) => {
                        if let Some(transversal) = transversal {
                            (
                                Ok(true),
                                Some(is_transversal_consistent(
                                    &transversal,
                                    graph,
                                    quotient_graph.encode_high(),
                                )),
                            )
                        } else {
                            (Ok(false), None)
                        }
                    }
                    Err(err) => (Err(err), None),
                }
            } else {
                let descriptive = solve(formula);
                (descriptive, None)
            }
        });
        kissat_time = k_time;
        descriptive = descriptive_validated.0;
        validated = descriptive_validated.1;

        matches!(descriptive, Ok(true))
    } else {
        // Trivially descriptive
        true
    };

    let quotient_handling_time = start_time.elapsed();
    let quotient_stats = QuotientStatistics {
        quotient_size,
        core_size: None,
        max_orbit_size,
        min_orbit_size,
        descriptive,
        validated,
        quotient_handling_time,
        kissat_time,
        orbit_gen_time,
        quotient_gen_time,
        encoding_time,
        orbit_sizes,
    };
    do_if_some(settings.get_stats(), |stats| {
        stats.log_quotient_statistic(quotient_stats);
        stats.log_iteration()
    });

    return_val
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient(
    generators_subset: &mut [Permutation],
    graph: &Graph,
    settings: &Settings,
) -> bool {
    let orbits = generate_orbits(generators_subset);

    let quotient_graph = QuotientGraph::from_graph_orbits(graph, orbits);

    let formula = encode_problem(&quotient_graph, graph);

    if let Some((formula, dict)) = formula {
        if settings.validate {
            let transversal_result = solve_validate(formula, dict);
            if let Some(transversal) = transversal_result.unwrap() {
                assert!(is_transversal_consistent(
                    &transversal,
                    graph,
                    quotient_graph.encode_high()
                ));
                true
            } else {
                false
            }
        } else {
            solve(formula).unwrap()
        }
    } else {
        true
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Error> {
    // Read the graph from a file or via CLI and ...
    let (mut graph, mut settings) = read_graph()?;

    if let Some(eval_buf) = settings.evaluate {
        let logs = evaluate_log_file(&mut eval_buf.lines());
        evaluate_logs(logs);
        return Ok(());
    }

    // Search for a non descriptive core in a single non-descriptive quotient.
    if settings.nondescriptive_core.is_some() {
        return search_with_core(&mut graph, &mut settings);
    }

    if settings.search_group {
        let nauty_graph = NautyGraph::from_graph(&mut graph);
        assert!(nauty_graph.check_valid());

        search_group(&mut graph, nauty_graph, &mut settings);
        return Ok(());
    }

    // ... compute the generators with nauty or Traces. Then ...
    let mut generators = compute_generators(&mut graph, &mut settings);

    do_if_some(settings.get_stats(), Statistics::log_nauty_done);
    do_if_some(settings.get_stats(), |st| {
        st.log_number_of_generators(generators.len())
    });

    // Sort the graph to allow easier lookup for edges.
    time!(graph_sort_time, _t, graph.sort());
    do_if_some(settings.get_stats(), |stats| {
        stats.log_graph_sorted(graph_sort_time)
    });

    if settings.gap_mode {
        return gap_mode(&graph, generators, settings.get_stats());
    }

    // ... iterate over the specified subsets of generators...
    if settings.get_stats().is_some() {
        // ... with statistics ...
        if settings.iter_powerset {
            generators
                .into_iter()
                .powerset()
                .skip(1)
                .find_map(|mut subset| {
                    if compute_quotient_with_statistics(&mut subset, &graph, &mut settings) {
                        Some(())
                    } else {
                        None
                    }
                });
        } else if !generators.is_empty() {
            compute_quotient_with_statistics(&mut generators, &graph, &mut settings);
        }

        do_if_some(settings.get_stats(), |statistics| {
            statistics.exhausted = true;
            statistics.log_end();
            statistics.save_statistics().unwrap();
        });
    } else {
        // ... or without.
        if settings.iter_powerset {
            generators
                .into_iter()
                .powerset()
                .skip(1)
                .find_map(|mut subset| {
                    if compute_quotient(&mut subset, &graph, &settings) {
                        Some(())
                    } else {
                        None
                    }
                });
        } else if !generators.is_empty() {
            compute_quotient(&mut generators, &graph, &settings);
        }
    }

    Ok(())
}
