use std::time::Instant;

use crate::{
    encoding::encode_problem,
    graph::Graph,
    graph::VertexIndex,
    permutation::Permutation,
    quotient::{generate_orbits, QuotientGraph},
    sat_solving::solve,
    statistics::{QuotientStatistics, Statistics},
    time, Error,
};

#[cfg(not(tarpaulin_include))]
pub fn check_class(graph: &Graph, representative_orbits: Vec<VertexIndex>) -> Result<bool, Error> {
    let quotient = QuotientGraph::from_graph_orbits(graph, representative_orbits);
    if let Some((formula, _)) = encode_problem(&quotient, graph) {
        solve(formula)
    } else {
        Ok(true)
    }
}

#[cfg(not(tarpaulin_include))]
pub fn check_class_stats(
    graph: &Graph,
    representative_group: &mut [Permutation],
    statistics: &mut Statistics,
) -> Result<bool, Error> {
    let start_time = Instant::now();

    time!(
        orbit_gen_time,
        orbits,
        generate_orbits(representative_group)
    );

    time!(
        quotient_gen_time,
        quotient,
        QuotientGraph::from_graph_orbits(graph, orbits)
    );
    let quotient_size = quotient.quotient_graph.size();

    let min_max_orbit_size = quotient.get_orbit_sizes();
    let (min_orbit_size, max_orbit_size) = min_max_orbit_size;

    time!(encoding_time, formula, encode_problem(&quotient, graph));

    time!(
        kissat_time,
        descriptive,
        if let Some((formula, _)) = formula {
            solve(formula)
        } else {
            Ok(true)
        }
    );

    let result = matches!(descriptive, Ok(true));

    let quotient_stats = QuotientStatistics {
        quotient_size,
        core_size: None,
        max_orbit_size,
        min_orbit_size,
        descriptive,
        validated: None,
        quotient_handling_time: start_time.elapsed(),
        kissat_time,
        orbit_gen_time,
        quotient_gen_time,
        encoding_time,
        orbit_sizes: Default::default(),
    };
    statistics.log_quotient_statistic(quotient_stats);
    statistics.log_iteration();

    Ok(result)
}
