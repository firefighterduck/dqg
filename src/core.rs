//! Different methods to destroy non-descriptive cores.

use crate::{
    debug::print_orbits_nauty_style,
    encoding::{encode_problem, OrbitEncoding},
    graph::Graph,
    quotient::{compute_generators, empty_orbits, generate_orbits, QuotientGraph},
    sat_solving::solve_mus_kitten,
    Error, Settings,
};

/// Just give every vertex* in the core a new color.
/// This breaks the core but changes the original graph.
///
/// *Well not really every vertex, but only those
/// in bigger orbits. We don't need to recolor single vertex orbits.
#[cfg(not(tarpaulin_include))]
fn recolor_core(graph: &mut Graph, core: &[OrbitEncoding]) -> Result<bool, Error> {
    let mut recolored = false;

    for orbit in core {
        if orbit.1.len() > 1 {
            for vertex in orbit.1.iter() {
                graph.recolor(*vertex)?;
                recolored = true;
            }
        }
    }

    Ok(recolored)
}

#[cfg(not(tarpaulin_include))]
pub fn search_with_core(graph: &mut Graph, settings: &Settings) -> Result<(), Error> {
    let mut generators;
    let mut orbits;
    let mut quotient_graph;
    let mut encoding;
    let mut counter = 0;

    loop {
        generators = compute_generators(graph, settings);
        if generators.is_empty() {
            println!("removed all symmetries in {} iterations", counter);
            if settings.output_orbits {
                print_orbits_nauty_style(empty_orbits(graph.size()), None);
            }
            return Ok(());
        }

        orbits = generate_orbits(&mut generators);
        graph.sort();
        quotient_graph = QuotientGraph::from_graph_orbits(graph, orbits);
        encoding = encode_problem(&quotient_graph, graph);

        if let Some((formula, dict)) = encoding {
            let next_core = solve_mus_kitten(formula, &quotient_graph, graph, dict)?;
            if let Some(core) = next_core {
                if !recolor_core(graph, &core.1)? {
                    println!("All colored, no more things to break");
                    break;
                }
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
