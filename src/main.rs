#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::{Either, Itertools};
use std::time::Instant;

mod graph;
use graph::{Graph, NautyGraph, SparseNautyGraph, TracesGraph};

mod input;
use input::read_graph;

mod quotient;
use quotient::{
    compute_generators_with_nauty, compute_generators_with_traces, empty_orbits, generate_orbits,
    search_group, Generator, Orbits, QuotientGraph,
};

mod encoding;
use encoding::{encode_problem, HighLevelEncoding};

mod sat_solving;
use sat_solving::solve;

mod parser;

mod statistics;
use statistics::{OrbitStatistics, QuotientStatistics, Statistics};

mod debug;
pub use debug::Error;
use debug::{print_formula, print_generator, print_orbits_nauty_style};

mod permutation;

mod metric;

mod transversal;
use transversal::is_transversal_consistent;

mod misc;
pub use misc::{do_if_some, MetricUsed, NautyTraces, Settings};

mod evaluate;
use evaluate::evaluate_log_file;

#[cfg(not(tarpaulin_include))]
fn compute_quotient_with_statistics(
    generators_subset: &mut [Generator],
    graph: &Graph,
    settings: &Settings,
    statistics: &mut Statistics,
) -> Option<QuotientGraph> {
    use crate::sat_solving::solve_validate;

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

    time!(
        _log_orbit_time,
        min_max_orbit_size,
        quotient_graph.get_orbit_sizes()
    );
    let (min_orbit_size, max_orbit_size) = min_max_orbit_size;

    time!(
        encoding_time,
        formula,
        encode_problem(&quotient_graph, graph)
    );

    if let Some((formula, dict)) = formula {
        if settings.print_formula {
            print_formula(formula);
            return None;
        }

        time!(kissat_time, descriptive_validated, {
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
                (solve(formula), None)
            }
        });
        let (descriptive, validated) = descriptive_validated;

        let return_val = if let Ok(true) = descriptive {
            Some(quotient_graph)
        } else {
            None
        };

        let quotient_handling_time = start_time.elapsed();
        let quotient_stats = QuotientStatistics {
            quotient_size,
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
        statistics.log_quotient_statistic(quotient_stats);

        return_val
    } else {
        eprintln!("Trivially descriptive");

        Some(quotient_graph)
    }
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient(
    generators_subset: &mut [Generator],
    graph: &Graph,
    settings: &Settings,
) -> Option<QuotientGraph> {
    use crate::sat_solving::solve_validate;

    let orbits = generate_orbits(generators_subset);

    let quotient_graph = QuotientGraph::from_graph_orbits(graph, orbits);

    let formula = encode_problem(&quotient_graph, graph);

    if let Some((formula, dict)) = formula {
        if settings.print_formula {
            print_formula(formula);
            return None;
        }

        if settings.validate {
            let transversal_result = solve_validate(formula, dict);
            if let Some(transversal) = transversal_result.unwrap() {
                assert!(is_transversal_consistent(
                    &transversal,
                    graph,
                    quotient_graph.encode_high()
                ));
                Some(quotient_graph)
            } else {
                None
            }
        } else {
            let descriptive = solve(formula);

            if descriptive.is_ok() && !descriptive.unwrap() {
                None
            } else {
                Some(quotient_graph)
            }
        }
    } else {
        eprintln!("Trivially descriptive");

        Some(quotient_graph)
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Error> {
    use std::io::BufRead;

    // Read the graph from a file or via CLI and ...
    let (mut graph, mut statistics, settings) = read_graph()?;

    if let Some(eval_buf) = settings.evaluate {
        let logs = evaluate_log_file(&mut eval_buf.lines());
        println!("{:#?}", logs);
        return Ok(());
    }

    let mut generators;

    if settings.search_group {
        let nauty_graph = NautyGraph::from_graph(&mut graph);
        assert!(nauty_graph.check_valid());

        search_group(&mut graph, nauty_graph, &settings);
        return Ok(());
    }

    // ... compute the generators with nauty or Traces. Then ...
    match settings.nauyt_or_traces {
        NautyTraces::Nauty => {
            let nauty_graph = NautyGraph::from_graph(&mut graph);

            assert!(nauty_graph.check_valid());
            generators = compute_generators_with_nauty(Either::Left(nauty_graph), &settings);
        }
        NautyTraces::SparseNauty => {
            let sparse_nauty_graph = SparseNautyGraph::from_graph(&mut graph);
            generators =
                compute_generators_with_nauty(Either::Right(sparse_nauty_graph), &settings);
        }
        NautyTraces::Traces => {
            let traces_graph = TracesGraph::from_graph(&mut graph);
            generators = compute_generators_with_traces(traces_graph, &settings);
        }
    };

    do_if_some(&mut statistics, Statistics::log_nauty_done);
    do_if_some(&mut statistics, |st| {
        st.log_number_of_generators(generators.len())
    });

    // Sort the graph to allow easier lookup for edges.
    time!(graph_sort_time, _t, graph.sort());

    // Apply a heuristic and find the "best" quotient according
    // to the chosen metric and print out the orbits for other
    // tools to directly use them.
    if settings.orbits_only {
        let orbits: Orbits;

        if generators.is_empty() {
            orbits = empty_orbits(graph.size());
        } else if let Some(metric) = settings.metric {
            // Heuristic: full search in set of quotients induced by generators
            orbits = generators
                .into_iter()
                .powerset()
                .skip(1)
                .filter_map(|mut subset| compute_quotient(&mut subset, &graph, &settings))
                .sorted_unstable_by(|left, right| metric.compare_quotients(left, right))
                .next()
                .map_or(empty_orbits(graph.size()), |quotient| quotient.orbits);
        } else {
            orbits = generate_orbits(&mut generators);
        }

        print_orbits_nauty_style(orbits);
        return Ok(());
    }

    // Search for a non descriptive core in a single non-descriptive quotient.
    if settings.nondescriptive_core {
        let core = generators
            .into_iter()
            .powerset()
            .skip(1)
            .find_map(|mut subset| {
                let orbits = generate_orbits(&mut subset);
                let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);
                let formula = encode_problem(&quotient_graph, &graph);
                if let Some((formula, _)) = formula {
                    if let Ok(false) = solve(formula) {
                        subset
                            .iter()
                            .for_each(|automorphism| print_generator(automorphism.clone()));
                        quotient_graph.search_non_descriptive_core(&graph)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Nondescriptive core can only be found for nondescriptive generator subsets!");
        println!("{:?}", core);
        return Ok(());
    }

    // ... iterate over the specified subsets of generators...
    if let Some(mut statistics) = statistics {
        statistics.log_graph_sorted(graph_sort_time);
        // ... with statistics ...
        if settings.iter_powerset {
            generators
                .into_iter()
                .powerset()
                .skip(1)
                .for_each(|mut subset| {
                    let _ = compute_quotient_with_statistics(
                        &mut subset,
                        &graph,
                        &settings,
                        &mut statistics,
                    );
                });
        } else if !generators.is_empty() {
            compute_quotient_with_statistics(&mut generators, &graph, &settings, &mut statistics);
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
                    let _ = compute_quotient(&mut subset, &graph, &settings);
                });
        } else if !generators.is_empty() {
            compute_quotient(&mut generators, &graph, &settings);
        }
    }

    Ok(())
}
