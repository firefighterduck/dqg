use kissat_rs::Solver;

use crate::{encoding::Clause, Error};

pub fn solve(formula: impl Iterator<Item = Clause>) -> Result<bool, Error> {
    Solver::decide_formula(formula).map_err(Error::from)
}

#[cfg(test)]
mod test {
    use crate::{
        encoding::{cache_graph_edges, encode_problem, EdgeCache},
        graph::Graph,
        quotient::QuotientGraph,
    };

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
        let mut cache = EdgeCache::new(graph.size());
        cache_graph_edges(&graph, &mut cache);

        // Not the actual orbits, but used to check for non-descriptiveness.
        let fake_orbits = vec![0, 1, 1, 3];
        let quotient = QuotientGraph::from_graph_orbits(&graph, fake_orbits);

        let formula = encode_problem(&quotient, &cache);

        let result = solve(formula.unwrap());
        assert!(result.is_ok());
        assert_eq!(false, result.unwrap());

        Ok(())
    }
}
