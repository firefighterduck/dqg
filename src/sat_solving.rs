use std::{
    collections::HashMap,
    convert::TryInto,
    process::{Command, Stdio},
    sync::Arc,
};

use itertools::Itertools;
use kissat_rs::{Assignment, Solver};
use num::ToPrimitive;

use crate::{
    debug::write_formula_dimacs,
    encoding::{Clause, QuotientGraphEncoding, SATEncodingDictionary},
    graph::VertexIndex,
    parser::parse_mus,
    Error,
};

pub fn solve(formula: impl Iterator<Item = Clause>) -> Result<bool, Error> {
    Solver::decide_formula(formula).map_err(Error::from)
}

fn get_transversal(
    assignment: HashMap<i32, Option<Assignment>>,
    dict: SATEncodingDictionary,
) -> Vec<(VertexIndex, VertexIndex)> {
    let mut picked = dict
        .destroy()
        .into_iter()
        .enumerate()
        .filter(|(literal, _)| {
            matches!(
                assignment.get(&(literal.to_i32().unwrap())),
                Some(Some(Assignment::True))
            )
        })
        .map(|(_, orbit_vertex)| orbit_vertex)
        .collect_vec();
    picked.sort_unstable_by(|(orbit1, _), (orbit2, _)| orbit1.cmp(orbit2));
    picked
}

pub fn solve_validate(
    formula: impl Iterator<Item = Clause>,
    dict: SATEncodingDictionary,
) -> Result<Option<Vec<(VertexIndex, VertexIndex)>>, Error> {
    let assignment = Solver::solve_formula(formula).map_err(Error::from)?;
    Ok(assignment.map(|assignment| get_transversal(assignment, dict)))
}

fn get_core_orbits(
    clause_indices: &[usize],
    formula: &[Clause],
    dict: SATEncodingDictionary,
) -> Vec<VertexIndex> {
    let mut core_orbits = Vec::new();
    let raw_dict = dict.destroy();

    for clause_index in clause_indices {
        let clause = formula
            .get(clause_index - 1)
            .expect("Clause index out of bounds!");
        for variable in clause {
            let orbit = raw_dict
                .get::<usize>(
                    variable
                        .abs()
                        .try_into()
                        .expect("Could not transform literal to usize!"),
                )
                .expect("Variable not in dict!")
                .0;
            core_orbits.push(orbit);
        }
    }

    core_orbits.sort_unstable();
    core_orbits.dedup();

    core_orbits
}

pub fn solve_mus(
    formula: impl Iterator<Item = Clause>,
    dict: SATEncodingDictionary,
) -> Result<Option<QuotientGraphEncoding>, Error> {
    let formula_collected = formula.collect_vec();

    if Solver::decide_formula(formula_collected.iter().cloned())? {
        Ok(None)
    } else {
        let mut mus = Command::new("picomus")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut stdin = mus.stdin.take().expect("Failed to open stdin to picomus!");

        let variable_number = dict.variable_number();
        let formula_arc = Arc::new(formula_collected);
        let closure_formula_arc = formula_arc.clone();
        std::thread::spawn(move || {
            write_formula_dimacs(&mut stdin, &closure_formula_arc, variable_number)
                .expect("Failed to write to stdin of picomus!")
        });
        let mus_out = mus.wait_with_output()?;

        // 20 for Unsatisfiable
        if mus_out.status.code() == Some(20) {
            let core = parse_mus(&mus_out.stdout)?;
            let core_orbits = get_core_orbits(&core, &formula_arc, dict);
            println!("{:?}", core_orbits);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use crate::{encoding::encode_problem, graph::Graph, quotient::QuotientGraph};

    use super::*;

    #[test]
    fn test_non_descriptive() -> Result<(), Error> {
        //0-1-2-3, where 1 and 2 are in the same (fake) orbit.
        let mut graph = Graph::new_ordered(4);
        graph.add_edge(0, 1)?;
        graph.add_edge(1, 2)?;
        graph.add_edge(2, 3)?;
        let colors = vec![1, 2, 2, 3];
        graph.set_colours(&colors)?;

        // Not the actual orbits, but used to check for non-descriptiveness.
        let fake_orbits = vec![0, 1, 1, 3];
        let quotient = QuotientGraph::from_graph_orbits(&graph, fake_orbits);

        let formula = encode_problem(&quotient, &graph);

        let result = solve(formula.unwrap().0);
        assert!(result.is_ok());
        assert_eq!(false, result.unwrap());

        Ok(())
    }
}
