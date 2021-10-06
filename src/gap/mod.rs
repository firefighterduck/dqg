use std::{
    process::{Command, Stdio},
    time::Instant,
};

use crate::{
    debug::print_orbits_nauty_style, graph::Graph, permutation::Permutation,
    quotient::generate_orbits, statistics::Statistics, Error,
};

mod print;
use print::write_gap_input;

mod parser;
use parser::parse_representatives;

mod search;
use search::{check_class, check_class_stats};

pub static GAP_IN_FILE: &str = "./dqg.g";

#[cfg(not(tarpaulin_include))]
pub fn gap_mode(
    graph: &Graph,
    mut generators: Vec<Permutation>,
    statistics: &mut Option<Statistics>,
) -> Result<(), Error> {
    if let Some(stats) = statistics {
        return gap_mode_statistics(graph, generators, stats);
    }

    // Early exit if full quotient is descriptive.
    let full_orbits = generate_orbits(&mut generators);
    if check_class(graph, full_orbits.clone())? {
        print_orbits_nauty_style(full_orbits, None);
        return Ok(());
    }

    write_gap_input(generators)?;

    let gap = Command::new("gap")
        .arg("-b")
        .arg("-o")
        .arg("16G")
        .arg("--nointeract")
        .arg(GAP_IN_FILE)
        .stdout(Stdio::piped())
        .spawn()?;

    let gap_out = gap.wait_with_output()?;

    if gap_out.status.success() {
        let representatives = parse_representatives(&gap_out.stdout, graph.size())?;
        for mut representative in representatives {
            let orbits = generate_orbits(&mut representative);
            if check_class(graph, orbits.clone())? {
                print_orbits_nauty_style(orbits, None);
                break;
            }
        }
    }

    Ok(())
}

#[cfg(not(tarpaulin_include))]
fn gap_mode_statistics(
    graph: &Graph,
    mut generators: Vec<Permutation>,
    statistics: &mut Statistics,
) -> Result<(), Error> {
    // Early exit if full quotient is descriptive.
    if let Some(orbits) = check_class_stats(graph, &mut generators, statistics)? {
        print_orbits_nauty_style(orbits, Some(statistics));
    } else {
        write_gap_input(generators)?;
        let before_gap_time = Instant::now();

        let gap = Command::new("gap")
            .arg("-b")
            .arg("-o")
            .arg("16G")
            .arg("--nointeract")
            .arg(GAP_IN_FILE)
            .stdout(Stdio::piped())
            .spawn()?;

        let gap_out = gap.wait_with_output()?;
        statistics.log_gap_done(before_gap_time.elapsed());

        if gap_out.status.success() {
            let representatives = parse_representatives(&gap_out.stdout, graph.size())?;
            for mut representative in representatives {
                if let Some(orbits) = check_class_stats(graph, &mut representative, statistics)? {
                    print_orbits_nauty_style(orbits, Some(statistics));
                    break;
                }
            }
        }
    }

    statistics.log_end();
    statistics.save_statistics()
}
