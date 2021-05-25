//! This file contains the high and low level encodings
//! needed to encode the descriptive quotient problem
//! as a CNF formula which can then be decided by a SAT solver.

use custom_debug_derive::Debug;
use itertools::Itertools;
use kissat_rs::Literal;
use std::collections::{HashMap, HashSet};

use crate::{
    graph::{Graph, VertexIndex},
    quotient::{Orbits, QuotientGraph},
};

pub type Clause = Vec<Literal>;
pub type Formula = Vec<Clause>;
const MAX_LITERAL: Literal = 2i32.pow(28) - 1;

/// Trait that defines whether a type can be encoded
/// into a high level view of a SAT formula.
pub trait HighLevelEncoding {
    type HighLevelRepresentation;
    fn encode_high(&self) -> Self::HighLevelRepresentation;
}

trait SATEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula;
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EdgeEncoding((VertexIndex, VertexIndex));

impl EdgeEncoding {
    pub fn get_edge(&self) -> &(VertexIndex, VertexIndex) {
        &self.0
    }
}

impl HighLevelEncoding for Graph {
    type HighLevelRepresentation = Vec<EdgeEncoding>;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        self.iterate_edges().map(EdgeEncoding).collect()
    }
}

pub type OrbitEncoding = (VertexIndex, Vec<VertexIndex>);

impl HighLevelEncoding for Orbits {
    type HighLevelRepresentation = Vec<OrbitEncoding>;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        self.iter()
            .enumerate()
            .sorted_by(|(_, orbit_a), (_, orbit_b)| orbit_a.cmp(orbit_b))
            .group_by(|(_, orbit)| **orbit)
            .into_iter()
            .map(|(orbit_number, vertices)| {
                (
                    orbit_number,
                    vertices
                        .into_iter()
                        .map(|(vertex, _)| vertex as VertexIndex)
                        .collect(),
                )
            })
            .collect()
    }
}

type QuotientGraphEncoding = (Vec<EdgeEncoding>, Vec<OrbitEncoding>);

impl HighLevelEncoding for QuotientGraph {
    type HighLevelRepresentation = QuotientGraphEncoding;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        (self.quotient_graph.encode_high(), self.orbits.encode_high())
    }
}

#[derive(Debug)]
pub struct SATEncodingDictionary {
    literal_counter: Literal,
    #[debug(skip)]
    literal_map: HashMap<Literal, Literal>,
    original_edge_cache: HashSet<EdgeEncoding>,
}

impl Default for SATEncodingDictionary {
    fn default() -> Self {
        SATEncodingDictionary {
            literal_counter: 1,
            literal_map: HashMap::new(),
            original_edge_cache: HashSet::new(),
        }
    }
}

impl SATEncodingDictionary {
    pub fn cache_edge(&mut self, edge: &EdgeEncoding) {
        self.original_edge_cache.insert(*edge);
    }

    pub fn lookup_edge(&self, start: &VertexIndex, end: &VertexIndex) -> bool {
        self.original_edge_cache
            .contains(&EdgeEncoding((*start, *end)))
    }

    /// Compute Cantor pairing and add if not stored in dict.
    fn pairing(&mut self, first: Literal, second: Literal) -> Literal {
        let pairing_result = (first + second) * (first + second + 1) / 2 + second;

        if let Some(literal) = self.literal_map.get(&pairing_result) {
            *literal
        } else {
            let literal = self.get_new_literal();
            self.literal_map.insert(pairing_result, literal);
            literal
        }
    }

    fn get_new_literal(&mut self) -> Literal {
        let new_literal = self.literal_counter;

        // Kissat doesn't allow variables over 2^28-1.
        assert!(new_literal < MAX_LITERAL);

        self.literal_counter += 1;
        new_literal
    }
}

impl SATEncoding for OrbitEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula {
        // This is actually the encoding that a valid transversal
        // can only choose one element from the orbit.

        // Encode the EO problem
        // Possible encodings:
        // - pairwise: (x1 || x2 || ... || xn) && for all i,j (~xi || ~xj), size = (n^2-n)/2
        // - bitwise: with aux vars, size = n*ceil(ld n), ceil(ld n) aux vars
        // - ladder: however this works, 3(n-1) binary clauses, n-1 ternary clauses, n-1 aux vars
        // - matrix: how the heck does this even, 2*sqrt(n) aux vars, 1 n-ary clause, 1 sqrt(n)-ary clause, 1 n/sqrt(n)-ary clause, 2n+4*sqrt(n)+O(fourth root n) binary clauses

        // For now we use pairwise encoding, because it's easy to implement
        let (orbit, orbit_elements) = self;
        let mut formula = Vec::new();
        let mut orbit_element_encodings = Vec::with_capacity(orbit_elements.len());

        for orbit_element in orbit_elements {
            orbit_element_encodings.push(dict.pairing(*orbit, *orbit_element));
        }

        // Pairwise mutual exclusion of orbit elements picked by the transversal.
        // Thus AT MOST ONE of these can be true.
        orbit_element_encodings
            .iter()
            .combinations(2)
            .for_each(|encoding_pair| {
                // -v1 || -v2; v1!=v2; v1, v2 in the given orbit
                formula.push(vec![-encoding_pair[0], -encoding_pair[1]]);
            });

        // Disjunction of all vertex-in-orbit pairs to encode AT LEAST ONE
        // ---------------------------------------------------------------
        // \/ vi for all vi in the orbit
        formula.push(orbit_element_encodings);

        // The EXACTLY ONE encoding for elements in the orbit picked by the transversal.
        formula
    }
}

impl SATEncoding for QuotientGraphEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula {
        // This is actually the encoding that edges between two
        // vertices (i.e. two orbits) of a quotient graph is preserved
        // when the transversal chooses two vertices from the orbits.
        let (quotient_edges, orbits) = self;
        let mut formula = Vec::new();

        // for all (o1,o2) edges in the quotient graph G\O (i.e. o1, o2 in O)
        for (start_orbit, end_orbit) in quotient_edges.iter().map(EdgeEncoding::get_edge) {
            let start_orbit_elements = orbits
                .iter()
                .find_map(|(orbit_number, orbit_elements)| {
                    if orbit_number == start_orbit {
                        Some(orbit_elements)
                    } else {
                        None
                    }
                })
                .expect(
                    "The edges were computed from the orbits, how can there be no fitting orbit?",
                );
            let end_orbit_elements = orbits
                .iter()
                .find_map(|(orbit_number, orbit_elements)| {
                    if orbit_number == end_orbit {
                        Some(orbit_elements)
                    } else {
                        None
                    }
                })
                .expect(
                    "The edges were computed from the orbits, how can there be no fitting orbit?",
                );

            // for all vertices v1 in o1
            for start_orbit_element in start_orbit_elements {
                // for all vertices v2 in o2
                'end: for end_orbit_element in end_orbit_elements {
                    // If the edge (v1,v2) for the two picked vertices exists
                    // in the original graph, we do not need to encode it.
                    if dict.lookup_edge(start_orbit_element, end_orbit_element) {
                        continue 'end;
                    }

                    let start_orbit_relation = dict.pairing(*start_orbit, *start_orbit_element);
                    let end_orbit_relation = dict.pairing(*end_orbit, *end_orbit_element);

                    // If there is an edge in the quotient graph,
                    // the transversal needs to pick vertices from
                    // the related orbits that are also connected in G.
                    // We don't actually need to encode this for existing edges
                    // in G but only for non-existing ones. We also don't need
                    // the edge in the quotient graph, as it also exists.
                    // ------------------------------------------------
                    // (o1,o2) && (o1, v1) && (o2,v2) => False
                    // ~(o1, v1) || ~(o2,v2)
                    let clause = vec![-start_orbit_relation, -end_orbit_relation];
                    formula.push(clause);
                }
            }
        }

        formula
    }
}

/// Encode the graph edges such that the dictionary
/// knows these and we can lookup whether an edge
/// exists and thus whether we need to add a clause to the formula.
fn cache_graph_edges(graph: &Graph, dict: &mut SATEncodingDictionary) {
    graph.encode_high().iter().for_each(|edge| {
        dict.cache_edge(edge);
    });
}

/// Encode the decision problem whether a set of generators
/// induces a descriptive quotient graph into SAT.
#[allow(clippy::needless_collect)]
pub fn encode_problem(
    graph: &Graph,
    quotient_graph: &QuotientGraph,
) -> Option<impl Iterator<Item = Clause>> {
    let mut dict = SATEncodingDictionary::default();
    cache_graph_edges(graph, &mut dict);

    let (quotient_edges, orbits) = quotient_graph.encode_high();

    let transversal_encoding = orbits
        .iter()
        .flat_map(|orbit| orbit.encode_sat(&mut dict))
        .collect::<Formula>();

    let descriptive_constraint_encoding = (quotient_edges, orbits).encode_sat(&mut dict);

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
    use crate::{graph::GraphError, Error};

    use super::*;

    #[test]
    fn test_encode_problem_trivial() -> Result<(), GraphError> {
        // 0 -- 1 -- 2 where 0 and 2 are in the same orbit
        let mut graph = Graph::new_ordered(3);
        graph.add_arc(0, 1)?;
        graph.add_arc(2, 1)?;
        let orbits = vec![0, 1, 0];
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);

        let formula = encode_problem(&graph, &quotient_graph);
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

        let formula = encode_problem(&graph, &quotient);
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

        let mut dict = SATEncodingDictionary::default();
        cache_graph_edges(&graph, &mut dict);

        assert!(dict.lookup_edge(&0, &1));
        assert!(dict.lookup_edge(&1, &2));
        assert!(dict.lookup_edge(&3, &4));
        assert!(dict.lookup_edge(&4, &0));

        Ok(())
    }

    #[test]
    fn test_descriptive_constraint() {
        let orbit_encoding = vec![(0, vec![0, 1]), (2, vec![2, 3])];
        let edge_encoding = vec![EdgeEncoding((0, 2))];
        let mut dict = SATEncodingDictionary::default();

        let o0v0 = dict.pairing(0, 0);
        let o0v1 = dict.pairing(0, 1);
        let o2v2 = dict.pairing(2, 2);
        let o2v3 = dict.pairing(2, 3);

        let constraint02 = vec![-o0v0, -o2v2];
        let constraint03 = vec![-o0v0, -o2v3];
        let constraint12 = vec![-o0v1, -o2v2];
        let constraint13 = vec![-o0v1, -o2v3];

        let formula = (edge_encoding, orbit_encoding).encode_sat(&mut dict);
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
        let pick0 = dict.pairing(0, 0);
        let pick1 = dict.pairing(0, 1);
        let pick4 = dict.pairing(0, 4);
        assert_eq!(1, pick0);
        assert_eq!(2, pick1);
        assert_eq!(3, pick4);

        let at_least_one = vec![pick0, pick1, pick4];
        let at_most_one = vec![
            vec![-pick0, -pick1],
            vec![-pick0, -pick4],
            vec![-pick1, -pick4],
        ];

        let formula = orbit_encoding.encode_sat(&mut dict);
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
