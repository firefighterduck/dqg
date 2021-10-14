//! Different methods to destroy non-descriptive cores.

use itertools::Itertools;
use std::time::{Duration, Instant};

use crate::{
    debug::print_orbits_nauty_style,
    do_if_some,
    encoding::{encode_problem, OrbitEncoding},
    graph::Graph,
    misc::CoreMetric,
    permutation::Permutation,
    quotient::{compute_generators, empty_orbits, generate_orbits, QuotientGraph},
    sat_solving::solve_mus_kitten,
    statistics::QuotientStatistics,
    time, time_assign, Error, Settings,
};

/// Just give every vertex* in the core a new color.
/// This breaks the core but changes the original graph.
///
/// *Well not really every vertex, but only those
/// in bigger orbits. We don't need to recolor single vertex orbits.
#[cfg(not(tarpaulin_include))]
fn recolor_core(graph: &mut Graph, core: &[OrbitEncoding]) -> Result<(), Error> {
    for orbit in core {
        for vertex in orbit.1.iter().skip(1) {
            graph.recolor(*vertex)?;
        }
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn search_with_core_recolor(graph: &mut Graph, settings: &mut Settings) -> Result<(), Error> {
    let mut generators;
    let mut orbits;
    let mut quotient_graph;
    let mut encoding;

    loop {
        let start_time = Instant::now();
        let mut kissat_time = Duration::ZERO;
        let mut core_size = None;

        time_assign!(nauty_time, generators, compute_generators(graph, settings));

        if generators.is_empty() {
            if settings.output_orbits {
                print_orbits_nauty_style(empty_orbits(graph.size()), None);
            }
            break;
        }

        time_assign!(orbit_gen_time, orbits, generate_orbits(&mut generators));

        time!(graph_sort_time, _sorted, graph.sort());

        time_assign!(
            quotient_gen_time,
            quotient_graph,
            QuotientGraph::from_graph_orbits(graph, orbits)
        );
        let quotient_size = quotient_graph.quotient_graph.size();
        let (min_orbit_size, max_orbit_size) = quotient_graph.get_orbit_sizes();

        time_assign!(
            encoding_time,
            encoding,
            encode_problem(&quotient_graph, graph)
        );

        let descriptive = if let Some((formula, dict)) = encoding {
            time!(
                kitten_time,
                next_core,
                solve_mus_kitten(formula, &quotient_graph, graph, dict)?
            );
            kissat_time = kitten_time;

            if let Some(core) = next_core {
                core_size = Some(core.1.len());
                // Break core with recoloring
                recolor_core(graph, &core.1)?;
                false
            } else {
                //Descriptive
                true
            }
        } else {
            // Trivially descriptive
            true
        };

        let quotient_handling_time = start_time.elapsed();
        let quotient_stats = QuotientStatistics {
            quotient_size,
            core_size,
            max_orbit_size,
            min_orbit_size,
            descriptive: Ok(descriptive),
            validated: None,
            quotient_handling_time,
            kissat_time,
            orbit_gen_time,
            quotient_gen_time,
            encoding_time,
            orbit_sizes: Default::default(),
        };
        do_if_some(settings.get_stats(), |stats| {
            stats.log_quotient_statistic(quotient_stats);
            stats.log_nauty_step(nauty_time);
            stats.log_graph_sorted_step(graph_sort_time);
            stats.log_iteration();
        });

        if descriptive {
            do_if_some(settings.get_stats(), |stats| stats.exhausted = true);
            if settings.output_orbits {
                print_orbits_nauty_style(quotient_graph.orbits, None);
            }
            break;
        }
    }

    do_if_some(settings.get_stats(), |stats| {
        stats.log_end();
        stats.save_statistics().unwrap();
    });

    Ok(())
}

/// Take the power of generators related to the core.
/// If a generator becomes the identity, it's removed.
#[cfg(not(tarpaulin_include))]
fn power_generators(generators: &mut [(Permutation, usize)], core: &[OrbitEncoding]) {
    for (generator, n) in generators {
        if *n == 0 {
            continue;
        }

        for (start, orbit) in core {
            let image = generator.evaluate(start);
            if let Some(image) = image {
                if image != *start && orbit.contains(&image) {
                    *n += 1;
                    break;
                }
            }
        }
    }
}

#[cfg(not(tarpaulin_include))]
fn search_with_core_power_generators(
    graph: &mut Graph,
    settings: &mut Settings,
) -> Result<(), Error> {
    let mut generators = compute_generators(graph, settings);
    let mut orig_generators = generators
        .iter()
        .cloned()
        .map(|perm| (perm, 1usize))
        .collect_vec();

    graph.sort();
    let mut orbits;
    let mut quotient_graph;
    let mut encoding;
    let mut counter = 0;

    loop {
        if orig_generators.is_empty() {
            println!("removed all symmetries in {} iterations", counter);
            if settings.output_orbits {
                print_orbits_nauty_style(empty_orbits(graph.size()), None);
            }
            return Ok(());
        }

        orbits = generate_orbits(&mut generators);
        quotient_graph = QuotientGraph::from_graph_orbits(graph, orbits);
        encoding = encode_problem(&quotient_graph, graph);

        if let Some((formula, dict)) = encoding {
            let next_core = solve_mus_kitten(formula, &quotient_graph, graph, dict)?;
            if let Some(core) = next_core {
                power_generators(&mut orig_generators, &core.1);
            } else {
                println!("Descriptive");
                break;
            }
        } else {
            println!("Trivially descriptive");
            break;
        }

        generators = orig_generators
            .iter_mut()
            .map(|(perm, n)| {
                let power = perm.nth_power_of(*n);
                if power.is_identity() {
                    *n = 0;
                }
                power
            })
            .collect_vec();
        orig_generators = orig_generators
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .collect();

        counter += 1;

        if counter > 30 {
            println!("Too many iterations.");
            break;
        }
    }

    if settings.output_orbits {
        print_orbits_nauty_style(quotient_graph.orbits, None);
    }
    println!("Took {} iterations", counter);
    Ok(())
}

/// Combine all related generators by composing them in order.
/// If there is only one generator related, remove it.
fn merge_generators(generators: Vec<Permutation>, core: &[OrbitEncoding]) -> Vec<Permutation> {
    let mut next_generators = Vec::new();

    let (involved, mut not_involved) =
        generators
            .into_iter()
            .partition::<Vec<Permutation>, _>(|generator| {
                for (start, orbit) in core {
                    let image = generator.evaluate(start);
                    if let Some(image) = image {
                        if image != *start && orbit.contains(&image) {
                            return true;
                        }
                    }
                }
                false
            });

    if involved.len() > 1 {
        let merged = involved
            .into_iter()
            .fold1(|first, second| first.merge(second).unwrap());
        if let Some(merged) = merged {
            next_generators.push(merged);
        }
    }

    next_generators.append(&mut not_involved);

    next_generators
}

#[cfg(not(tarpaulin_include))]
fn search_with_core_merge_generators(
    graph: &mut Graph,
    settings: &mut Settings,
) -> Result<(), Error> {
    let mut generators = compute_generators(graph, settings);
    graph.sort();
    let mut orbits;
    let mut quotient_graph;
    let mut encoding;
    let mut counter = 0;

    loop {
        if generators.is_empty() {
            println!("removed all symmetries in {} iterations", counter);
            if settings.output_orbits {
                print_orbits_nauty_style(empty_orbits(graph.size()), None);
            }
            return Ok(());
        }

        orbits = generate_orbits(&mut generators);
        quotient_graph = QuotientGraph::from_graph_orbits(graph, orbits);
        encoding = encode_problem(&quotient_graph, graph);

        if let Some((formula, dict)) = encoding {
            let next_core = solve_mus_kitten(formula, &quotient_graph, graph, dict)?;
            if let Some(core) = next_core {
                generators = merge_generators(generators, &core.1);
            } else {
                println!("Descriptive");
                break;
            }
        } else {
            println!("Trivially descriptive");
            break;
        }

        counter += 1;
    }

    if settings.output_orbits {
        print_orbits_nauty_style(quotient_graph.orbits, None);
    }
    println!("Took {} iterations", counter);
    Ok(())
}

#[cfg(not(tarpaulin_include))]
pub fn search_with_core(graph: &mut Graph, settings: &mut Settings) -> Result<(), Error> {
    match settings.nondescriptive_core {
        Some(CoreMetric::Recolor) => search_with_core_recolor(graph, settings),
        Some(CoreMetric::PowerGenerators) => search_with_core_power_generators(graph, settings),
        Some(CoreMetric::MergeGenerators) => search_with_core_merge_generators(graph, settings),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_merge_generators() {
        let generators = vec![
            Permutation::new(vec![0, 1, 2, 3, 5, 4]),
            Permutation::new(vec![0, 2, 1, 3, 4, 5]),
            Permutation::new(vec![0, 2, 1, 4, 3, 5]),
            Permutation::new(vec![0, 5, 2, 4, 3, 1]),
        ];
        let core = vec![(3, vec![3, 4])];

        let expected = vec![
            Permutation::new(vec![0, 2, 5, 3, 4, 1]),
            Permutation::new(vec![0, 1, 2, 3, 5, 4]),
            Permutation::new(vec![0, 2, 1, 3, 4, 5]),
        ];
        let merged = merge_generators(generators, &core);
        assert_eq!(expected, merged);

        let generators = vec![
            Permutation::new(vec![0, 1, 2, 3, 5, 4]),
            Permutation::new(vec![0, 2, 1, 3, 4, 5]),
            Permutation::new(vec![0, 2, 1, 4, 3, 5]),
            Permutation::new(vec![0, 5, 3, 2, 4, 1]),
        ];
        let core = vec![(3, vec![3, 4])];

        let expected = vec![
            Permutation::new(vec![0, 1, 2, 3, 5, 4]),
            Permutation::new(vec![0, 2, 1, 3, 4, 5]),
            Permutation::new(vec![0, 5, 3, 2, 4, 1]),
        ];
        let merged = merge_generators(generators, &core);
        assert_eq!(expected, merged);
    }
}
