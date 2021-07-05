#![warn(rust_2018_idioms)]
//#![deny(warnings, missing_docs)]

//! Project to find heuristics for
//! descriptive quotients of graphs
//! for certain conditions.

use itertools::{Either, Itertools};
use std::{str::FromStr, time::Instant};

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
use debug::{print_formula, print_generator, print_orbits_nauty_style, MetricError};

mod permutation;

mod metric;
use metric::{BiggestOrbits, LeastOrbits, Metric, Sparsity};

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

#[derive(Debug, Clone, Copy)]
pub enum MetricUsed {
    LeastOrbits,
    BiggestOrbits,
    Sparsity,
}

impl MetricUsed {
    pub fn compare_quotients(
        &self,
        left: &QuotientGraph,
        right: &QuotientGraph,
    ) -> std::cmp::Ordering {
        match &self {
            Self::LeastOrbits => LeastOrbits::compare_quotients(left, right),
            Self::BiggestOrbits => BiggestOrbits::compare_quotients(left, right),
            Self::Sparsity => Sparsity::compare_quotients(left, right),
        }
    }
}

impl FromStr for MetricUsed {
    type Err = MetricError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("least_orbits") {
            Ok(Self::LeastOrbits)
        } else if s.starts_with("biggest_orbit") {
            Ok(Self::BiggestOrbits)
        } else if s.starts_with("sparsity") {
            Ok(Self::Sparsity)
        } else {
            Err(MetricError(s.to_string()))
        }
    }
}

impl Default for MetricUsed {
    fn default() -> Self {
        Self::LeastOrbits
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
    /// Search for the smallest non-descriptive quotient
    /// core in the first non-descriptive quotient graph.
    pub nondescriptive_core: bool,
    /// Search in the whole automorphism group instead
    /// of a set of generators.
    pub search_group: bool,
    /// Use the given metric to find the "best" quotient
    /// and use it as described by the other flags.
    pub metric: Option<MetricUsed>,
    ///  Call nauty or traces.
    pub nauyt_or_traces: NautyTraces,
}

#[cfg(not(tarpaulin_include))]
fn compute_quotient_with_statistics(
    generators_subset: &mut [Generator],
    graph: &Graph,
    settings: &Settings,
    statistics: &mut Statistics,
) -> Option<QuotientGraph> {
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
        QuotientGraph::from_graph_orbits(&graph, orbits)
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

    if let Some(formula) = formula {
        if settings.print_formula {
            print_formula(formula);
            return None;
        }

        time!(kissat_time, descriptive, solve(formula));
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
    let orbits = generate_orbits(generators_subset);

    let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);

    let formula = encode_problem(&quotient_graph, graph);

    if let Some(formula) = formula {
        if settings.print_formula {
            print_formula(formula);
            return None;
        }

        let descriptive = solve(formula);

        if descriptive.is_ok() && !descriptive.unwrap() {
            None
        } else {
            Some(quotient_graph)
        }
    } else {
        eprintln!("Trivially descriptive");
        Some(quotient_graph)
    }
}

#[cfg(not(tarpaulin_include))]
fn main() -> Result<(), Error> {
    // Read the graph from a file or via CLI and ...
    let (mut graph, mut statistics, settings) = read_graph()?;
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
                if let Some(formula) = formula {
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
