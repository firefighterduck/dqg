use std::{
    collections::HashMap,
    convert::TryInto,
    fs::File,
    process::{Command, Stdio},
    sync::Arc,
};

use flussab_cnf::cnf::Parser;
use itertools::Itertools;
use kissat_rs::{Assignment, Solver};
use num::ToPrimitive;

use crate::{
    debug::write_formula_dimacs,
    encoding::{
        encode_problem, Clause, HighLevelEncoding, QuotientGraphEncoding, SATEncodingDictionary,
    },
    graph::{Graph, VertexIndex},
    parser::parse_mus,
    quotient::QuotientGraph,
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

#[cfg(not(tarpaulin_include))]
pub fn solve_validate(
    formula: impl Iterator<Item = Clause>,
    dict: SATEncodingDictionary,
) -> Result<Option<Vec<(VertexIndex, VertexIndex)>>, Error> {
    let assignment = Solver::solve_formula(formula).map_err(Error::from)?;
    Ok(assignment.map(|assignment| get_transversal(assignment, dict)))
}

fn get_core_orbits_indexed(
    clause_indices: &[usize],
    formula: &[Clause],
    dict: SATEncodingDictionary,
) -> Vec<VertexIndex> {
    let core_formula = clause_indices
        .iter()
        .map(|index| {
            formula
                .get(index - 1)
                .expect("Clause index out of range!")
                .clone()
        })
        .collect_vec();
    get_core_orbits(&core_formula, dict)
}

fn get_core_orbits(core_formula: &[Clause], dict: SATEncodingDictionary) -> Vec<VertexIndex> {
    let mut core_orbits = Vec::new();
    let raw_dict = dict.destroy();

    for clause in core_formula {
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

#[cfg(not(tarpaulin_include))]
pub fn solve_mus(
    formula: impl Iterator<Item = Clause>,
    quotient_graph: &QuotientGraph,
    graph: &Graph,
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
            let core_orbits = get_core_orbits_indexed(&core, &formula_arc, dict);
            dbg!(&core_orbits);
            let sub_quotient = quotient_graph.induced_subquotient(&core_orbits)?;

            // Make sure that the found orbits are in fact a non-descriptive core.
            // I don't really doubt picmus, but who knows what kind of MUS it finds.
            let (formula, _) = encode_problem(&sub_quotient, graph).unwrap();
            assert!(matches!(solve(formula), Ok(false)));

            Ok(Some(sub_quotient.encode_high()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(not(tarpaulin_include))]
pub fn solve_mus_kitten(
    formula: impl Iterator<Item = Clause>,
    quotient_graph: &QuotientGraph,
    graph: &Graph,
    dict: SATEncodingDictionary,
) -> Result<Option<QuotientGraphEncoding>, Error> {
    let formula_collected = formula.collect_vec();

    if Solver::decide_formula(formula_collected.iter().cloned())? {
        Ok(None)
    } else {
        let mut dqg_file = File::create("./dqg.cnf")?;
        let variable_number = dict.variable_number();
        write_formula_dimacs(&mut dqg_file, &formula_collected, variable_number)?;

        let mut kitten = Command::new("./kitten")
            .arg("-O25")
            .arg("./dqg.cnf")
            .arg("./core.cnf")
            .stdout(Stdio::piped())
            .spawn()?;
        let kitten_exit = kitten.wait()?;

        // 20 for Unsatisfiable
        if kitten_exit.code() == Some(20) {
            let core_file = File::open("./core.cnf")?;
            let mut core_parser = Parser::from_read(core_file, true).unwrap();
            let mut core: Vec<Vec<VertexIndex>> = Vec::new();

            loop {
                let next = core_parser.next_clause().unwrap();
                match next {
                    Some(clause) => core.push(clause.to_vec()),
                    None => break,
                }
            }

            let core_orbits = get_core_orbits(&core, dict);
            let sub_quotient = quotient_graph.induced_subquotient(&core_orbits)?;

            // Make sure that the found orbits are in fact a non-descriptive core.
            // I don't really doubt picmus, but who knows what kind of MUS it finds.
            let (formula, _) = encode_problem(&sub_quotient, graph).unwrap();
            assert!(matches!(solve(formula), Ok(false)));

            Ok(Some(sub_quotient.encode_high()))
        } else {
            Ok(None)
        }
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

    #[test]
    fn test_get_transversal() {
        // Orbit 1: {0,1}
        // orbit 2: {2,3}
        // Edges: 0-2,1-3
        // Transversal: 0|->0,2|->3

        let mut assignment = HashMap::new();
        assignment.insert(1, Some(Assignment::True));
        assignment.insert(2, Some(Assignment::False));
        assignment.insert(3, Some(Assignment::True));
        assignment.insert(4, Some(Assignment::False));

        let mut dict = SATEncodingDictionary::default();
        assert_eq!(1, dict.lookup_pairing(0, 0));
        assert_eq!(2, dict.lookup_pairing(0, 1));
        assert_eq!(3, dict.lookup_pairing(2, 3));
        assert_eq!(4, dict.lookup_pairing(2, 2));

        let expected_transversal = vec![(0, 0), (2, 3)];
        assert_eq!(expected_transversal, get_transversal(assignment, dict));
    }

    #[test]
    fn test_get_core_orbits_indexed() {
        let mut dict = SATEncodingDictionary::default();
        let pairs = vec![
            (14, 14),
            (14, 34),
            (22, 22),
            (22, 26),
            (134, 144),
            (134, 134),
            (154, 154),
            (154, 158),
            (127, 127),
        ];
        for (index, (orbit, vertex)) in pairs.into_iter().enumerate() {
            assert_eq!(index as VertexIndex + 1, dict.lookup_pairing(orbit, vertex));
        }

        let formula = vec![vec![1, 2], vec![-1, -2], vec![3, 4], vec![-3, -4], vec![9]];
        let core = vec![1, 3, 5];

        let expected_orbits = vec![14, 22, 127];

        assert_eq!(
            expected_orbits,
            get_core_orbits_indexed(&core, &formula, dict)
        );
    }
}
