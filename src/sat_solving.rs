use std::collections::HashMap;

use itertools::Itertools;
use kissat_rs::{Assignment, Solver};
use num::ToPrimitive;

use crate::{
    encoding::{Clause, SATEncodingDictionary},
    graph::VertexIndex,
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
