//! This file contains the high and low level encodings
//! needed to encode the descriptive quotient problem
//! as a CNF formula which can then be decided by a SAT solver.

use kissat_rs::Literal;

mod encoding_dict;
use encoding_dict::SATEncodingDictionary;

mod high_level;
use high_level::QuotientGraphEncoding;
pub use high_level::{HighLevelEncoding, OrbitEncoding};

mod low_level;
use low_level::SATEncoding;

use crate::{graph::Graph, quotient::QuotientGraph};

pub type Clause = Vec<Literal>;
pub type Formula = Vec<Clause>;

/// Encode the decision problem whether a set of generators
/// induces a descriptive quotient graph into SAT.
#[allow(clippy::needless_collect)]
pub fn encode_problem(
    quotient_graph: &QuotientGraph,
    original_graph: &Graph,
) -> Option<impl Iterator<Item = Clause>> {
    let mut dict = SATEncodingDictionary::default();

    let QuotientGraphEncoding(quotient_edges, orbits) = quotient_graph.encode_high();

    let transversal_encoding = orbits
        .iter()
        .flat_map(|orbit| orbit.encode_sat(&mut dict, original_graph))
        .collect::<Formula>();

    let descriptive_constraint_encoding =
        QuotientGraphEncoding(quotient_edges, orbits).encode_sat(&mut dict, original_graph);

    if descriptive_constraint_encoding.is_empty() {
        None
    } else {
        Some(
            transversal_encoding
                .into_iter()
                .chain(descriptive_constraint_encoding.into_iter()),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{encoding::high_level::EdgeEncoding, graph::GraphError, Error};

    use super::*;

    #[test]
    fn test_encode_problem_trivial() -> Result<(), GraphError> {
        // 0 -- 1 -- 2 where 0 and 2 are in the same orbit
        let mut graph = Graph::new_ordered(3);
        graph.add_arc(0, 1)?;
        graph.add_arc(2, 1)?;
        let orbits = vec![0, 1, 0];
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);

        let formula = encode_problem(&quotient_graph, &graph);
        assert!(formula.is_none());
        Ok(())
    }

    #[test]
    fn test_encode_problem_nontrivial() -> Result<(), GraphError> {
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

        let expected: Formula = vec![
            // vertex 0 in orbit 0
            vec![1],
            // Exactly one of 1,2 in orbit 1
            vec![-2, -3],
            vec![2, 3],
            // vertex 3 in orbit 3
            vec![4],
            // can't pick both 0 in 0 and 2 in 1
            vec![-1, -3],
            // can't pick both 2 in 1 and 0 in 0
            vec![-3, -1],
            // can't pick both 1 in 1 and 3 in 3
            vec![-2, -4],
            // can't pick both 3 in 3 and 1 in 1
            vec![-4, -2],
        ];

        let formula = encode_problem(&quotient, &graph);
        assert!(formula.is_some());
        assert!(formula
            .unwrap()
            .zip(expected.into_iter())
            .all(|(fst, snd)| fst == snd));

        Ok(())
    }

    #[test]
    fn test_encode_graph_edges() -> Result<(), Error> {
        let mut graph = Graph::new_ordered(5);
        graph.add_arc(0, 1)?;
        graph.add_arc(1, 2)?;
        graph.add_arc(3, 4)?;
        graph.add_arc(4, 0)?;

        assert_eq!(true, graph.lookup_edge(&0, &1));
        assert_eq!(true, graph.lookup_edge(&1, &2));
        assert_eq!(true, graph.lookup_edge(&3, &4));
        assert_eq!(true, graph.lookup_edge(&4, &0));

        Ok(())
    }

    #[test]
    fn test_descriptive_constraint() {
        let orbit_encoding = vec![(0, vec![0, 1]), (2, vec![2, 3])];
        let edge_encoding = vec![EdgeEncoding((0, 2))];
        let mut dict = SATEncodingDictionary::default();
        let some_graph = Graph::new_ordered(4);

        let o0v0 = dict.lookup_pairing(0, 0);
        let o0v1 = dict.lookup_pairing(0, 1);
        let o2v2 = dict.lookup_pairing(2, 2);
        let o2v3 = dict.lookup_pairing(2, 3);

        let constraint02 = vec![-o0v0, -o2v2];
        let constraint03 = vec![-o0v0, -o2v3];
        let constraint12 = vec![-o0v1, -o2v2];
        let constraint13 = vec![-o0v1, -o2v3];

        let formula =
            QuotientGraphEncoding(edge_encoding, orbit_encoding).encode_sat(&mut dict, &some_graph);
        assert_eq!(4, formula.len());
        assert!(formula.contains(&constraint02));
        assert!(formula.contains(&constraint03));
        assert!(formula.contains(&constraint12));
        assert!(formula.contains(&constraint13));
    }

    #[test]
    fn test_transversal_encoding() {
        let orbit_encoding = (0, vec![0, 1, 4]);
        let mut dict = SATEncodingDictionary::default();
        let some_graph = Graph::new_ordered(0);
        let pick0 = dict.lookup_pairing(0, 0);
        let pick1 = dict.lookup_pairing(0, 1);
        let pick4 = dict.lookup_pairing(0, 4);
        assert_eq!(1, pick0);
        assert_eq!(2, pick1);
        assert_eq!(3, pick4);

        let at_least_one = vec![pick0, pick1, pick4];
        let at_most_one = vec![
            vec![-pick0, -pick1],
            vec![-pick0, -pick4],
            vec![-pick1, -pick4],
        ];

        let formula = orbit_encoding.encode_sat(&mut dict, &some_graph);
        assert_eq!(4, formula.len());
        assert!(formula.contains(&at_least_one));
        for mut_ex in at_most_one {
            assert!(formula.contains(&mut_ex));
        }
    }

    #[test]
    fn test_encode_graph() {
        let mut graph = Graph::new_ordered(4);
        graph.add_arc(0, 1).unwrap();
        graph.add_arc(1, 2).unwrap();
        graph.add_arc(2, 3).unwrap();
        graph.add_arc(3, 1).unwrap();
        let encoded = graph.encode_high();
        assert_eq!(
            encoded,
            vec![
                EdgeEncoding((0, 1)),
                EdgeEncoding((1, 2)),
                EdgeEncoding((2, 3)),
                EdgeEncoding((3, 1))
            ]
        );
    }

    #[test]
    fn test_encode_orbits() {
        let orbits = vec![0, 1, 2, 0, 2, 1, 0];
        let encoded = orbits.encode_high();
        assert_eq!(
            encoded,
            vec![(0, vec![0, 3, 6]), (1, vec![1, 5]), (2, vec![2, 4])]
        );
    }
}
