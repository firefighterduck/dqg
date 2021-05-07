//! This file contains the high and low level encodings
//! needed to encode the descriptive quotient problem
//! as a CNF formula which can then be decided by a SAT solver.

use std::collections::HashMap;

use itertools::Itertools;
use kissat_rs::Literal;

use crate::{
    graph::{Graph, VertexIndex},
    quotient::{Orbits, QuotientGraph},
};

pub type Clause = Vec<Literal>;
pub type Formula = Vec<Clause>;
const MAX_LITERAL: Literal = 2i32.pow(28) - 1;

/// Trait that defines whether a type can be encoded
/// into a high level view of a SAT formula.
trait HighLevelEncoding {
    type HighLevelRepresentation;
    fn encode_high(&self, in_quotient: bool) -> Self::HighLevelRepresentation;
}

trait SATEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula;
}

#[derive(Debug, PartialEq, Eq)]
enum EdgeEncoding {
    OrbitEdge((VertexIndex, VertexIndex)),
    VertexEdge((VertexIndex, VertexIndex)),
}

impl EdgeEncoding {
    pub fn get_edge(&self) -> &(VertexIndex, VertexIndex) {
        match self {
            EdgeEncoding::OrbitEdge(edge) => edge,
            EdgeEncoding::VertexEdge(edge) => edge,
        }
    }
}

impl HighLevelEncoding for Graph {
    type HighLevelRepresentation = Vec<EdgeEncoding>;

    fn encode_high(&self, in_quotient: bool) -> Self::HighLevelRepresentation {
        let mut edges = Vec::new();

        self.iterate_edges(|edge| {
            edges.push(edge);
        });

        edges
            .into_iter()
            .map(|edge| {
                if in_quotient {
                    EdgeEncoding::OrbitEdge(edge)
                } else {
                    EdgeEncoding::VertexEdge(edge)
                }
            })
            .collect()
    }
}

type OrbitEncoding = (VertexIndex, Vec<VertexIndex>);

impl HighLevelEncoding for Orbits {
    type HighLevelRepresentation = Vec<OrbitEncoding>;

    fn encode_high(&self, _in_quotient: bool) -> Self::HighLevelRepresentation {
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

    fn encode_high(&self, _in_quotient: bool) -> Self::HighLevelRepresentation {
        (
            self.quotient_graph.encode_high(true),
            self.orbits.encode_high(true),
        )
    }
}

#[derive(Debug)]
struct SATEncodingDictionary {
    literal_counter: Literal,
    vertex_map: HashMap<VertexIndex, Literal>,
    orbit_map: HashMap<VertexIndex, Literal>,
}

impl SATEncodingDictionary {
    pub fn new() -> Self {
        SATEncodingDictionary {
            literal_counter: 1,
            vertex_map: HashMap::new(),
            orbit_map: HashMap::new(),
        }
    }

    pub fn lookup_vertex(&self, vertex: &VertexIndex) -> Option<Literal> {
        self.vertex_map.get(vertex).map(|literal_ref| *literal_ref)
    }

    pub fn lookup_orbit(&self, orbit: &VertexIndex) -> Option<Literal> {
        self.orbit_map.get(orbit).map(|literal_ref| *literal_ref)
    }

    pub fn lookup_edge(&mut self, edge: &EdgeEncoding) -> Option<Literal> {
        let start_lit;
        let end_lit;
        match edge {
            EdgeEncoding::OrbitEdge((start, end)) => {
                start_lit = self.lookup_orbit(start);
                end_lit = self.lookup_orbit(end);
            }
            EdgeEncoding::VertexEdge((start, end)) => {
                start_lit = self.lookup_vertex(start);
                end_lit = self.lookup_vertex(end);
            }
        };

        start_lit
            .zip(end_lit)
            .map(|(start, end)| self.pairing(start, end))
    }

    pub fn lookup_or_add_vertex(&mut self, vertex: &VertexIndex) -> Literal {
        if let Some(literal) = self.vertex_map.get(vertex) {
            *literal
        } else {
            let new_lit = self.get_new_literal();
            self.vertex_map.insert(*vertex, new_lit);
            new_lit
        }
    }

    pub fn lookup_or_add_orbit(&mut self, orbit: &VertexIndex) -> Literal {
        if let Some(literal) = self.orbit_map.get(orbit) {
            *literal
        } else {
            let new_lit = self.get_new_literal();
            self.orbit_map.insert(*orbit, new_lit);
            new_lit
        }
    }

    pub fn lookup_or_add_edge(&mut self, edge: &EdgeEncoding) -> Literal {
        if let Some(literal) = self.lookup_edge(edge) {
            literal
        } else {
            let start_lit;
            let end_lit;
            match edge {
                EdgeEncoding::OrbitEdge((start, end)) => {
                    start_lit = self.lookup_or_add_orbit(start);
                    end_lit = self.lookup_or_add_orbit(end);
                }
                EdgeEncoding::VertexEdge((start, end)) => {
                    start_lit = self.lookup_or_add_vertex(start);
                    end_lit = self.lookup_or_add_vertex(end);
                }
            }

            self.pairing(start_lit, end_lit)
        }
    }

    /// This computes the Cantor pairing function for th two given literals.
    fn pairing(&self, first: Literal, second: Literal) -> Literal {
        let pairing_result = (first + second) * (first + second + 1) / 2 + second;
        // The return value must also be a valid literal.
        // Whereas the normally assigned literals grow from 0,
        // the paired ones grow from the positive max value to reduce collisions.
        // For some reason or other, the max literal for Kissat
        // is 2^28-1. Thus, this is used.
        assert!(
            self.literal_counter < MAX_LITERAL - pairing_result,
            "SAT vertex variable space and pair variable space intersect!"
        );

        MAX_LITERAL - pairing_result
    }

    fn get_new_literal(&mut self) -> Literal {
        let new_literal = self.literal_counter;
        self.literal_counter += 1;
        new_literal
    }
}

impl SATEncoding for EdgeEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula {
        let edge_literal = dict.lookup_or_add_edge(self);
        vec![vec![edge_literal]]
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

        let orbit_encoding = dict.lookup_or_add_orbit(&orbit);

        for orbit_element in orbit_elements {
            let element_encoding = dict.lookup_or_add_vertex(orbit_element);
            orbit_element_encodings.push(dict.pairing(orbit_encoding, element_encoding));
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
            let start_encoding = dict.lookup_or_add_orbit(start_orbit);
            let end_encoding = dict.lookup_or_add_orbit(end_orbit);
            let edge_encoding = dict.pairing(start_encoding, end_encoding);
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
                let start_element_encoding = dict.lookup_or_add_vertex(start_orbit_element);
                // for all vertices v2 in o2
                for end_orbit_element in end_orbit_elements {
                    let end_element_encoding = dict.lookup_or_add_vertex(end_orbit_element);
                    let start_orbit_relation = dict.pairing(start_encoding, start_element_encoding);
                    let end_orbit_relation = dict.pairing(end_encoding, end_element_encoding);
                    let original_edge_encoding =
                        dict.pairing(start_element_encoding, end_element_encoding);

                    // If there is an edge in the quotient graph,
                    // the transversal needs to pick vertices from
                    // the related orbits that are also connected in G.
                    // ------------------------------------------------
                    // (o1,o2) && (o1, v1) && (o2,v2) => (v1,v2)
                    // ~(o1,o2) || ~(o1, v1) || ~(o2,v2) || (v1,v2)
                    let clause = vec![
                        -edge_encoding,
                        -start_orbit_relation,
                        -end_orbit_relation,
                        original_edge_encoding,
                    ];
                    formula.push(clause);
                }
            }
        }

        formula
    }
}

/// Encode the decision problem whether a set of generators
/// induces a descriptive quotient graph into SAT.
pub fn encode_problem(graph: &Graph, quotient_graph: &QuotientGraph) -> Formula {
    let mut dict = SATEncodingDictionary::new();

    let mut graph_edges_encoding: Formula = graph
        .encode_high(false)
        .into_iter()
        .flat_map(|edge| edge.encode_sat(&mut dict))
        .collect();
    let (quotient_edges, orbits) = quotient_graph.encode_high(true);
    let mut quotient_edges_encoding: Formula = quotient_edges
        .iter()
        .flat_map(|edge| edge.encode_sat(&mut dict))
        .collect();
    let mut transversal_encoding: Formula = orbits
        .iter()
        .flat_map(|orbit| orbit.encode_sat(&mut dict))
        .collect();
    let mut descriptive_constraint_encoding = (quotient_edges, orbits).encode_sat(&mut dict);

    graph_edges_encoding.append(&mut quotient_edges_encoding);
    graph_edges_encoding.append(&mut transversal_encoding);
    graph_edges_encoding.append(&mut descriptive_constraint_encoding);

    graph_edges_encoding
}

#[cfg(test)]
mod test {
    use crate::graph::GraphError;

    use super::*;

    #[test]
    fn test_encode_problem() -> Result<(), GraphError> {
        // 0 -- 1 -- 2 where 0 and 2 are in the same orbit
        let mut graph = Graph::new_ordered(3);
        graph.add_arc(0, 1)?;
        graph.add_arc(2, 1)?;
        let orbits = vec![0, 1, 0];
        let quotient_graph = QuotientGraph::from_graph_orbits(&graph, orbits);
        let dict = SATEncodingDictionary::new();

        // Expected mappings: vertices: 0->1, 1->2, 2->3, orbits 0->4, 1->5
        let vert_enc0 = 1;
        let vert_enc1 = 2;
        let vert_enc2 = 3;
        let orb_enc0 = 4;
        let orb_enc1 = 5;

        let expected_formula = vec![
            // Graph edges
            vec![dict.pairing(vert_enc0, vert_enc1)],
            vec![dict.pairing(vert_enc2, vert_enc1)],
            //Quotient graph edges
            vec![dict.pairing(orb_enc0, orb_enc1)],
            // Transversal for orbit 0
            // Not both
            vec![
                -dict.pairing(orb_enc0, vert_enc0),
                -dict.pairing(orb_enc0, vert_enc2),
            ],
            // Either of these
            vec![
                dict.pairing(orb_enc0, vert_enc0),
                dict.pairing(orb_enc0, vert_enc2),
            ],
            // Transversal for orbit 1
            vec![dict.pairing(orb_enc1, vert_enc1)],
            // Descriptive constraint
            vec![
                -dict.pairing(orb_enc0, orb_enc1),
                -dict.pairing(orb_enc0, vert_enc0),
                -dict.pairing(orb_enc1, vert_enc1),
                dict.pairing(vert_enc0, vert_enc1),
            ],
            vec![
                -dict.pairing(orb_enc0, orb_enc1),
                -dict.pairing(orb_enc0, vert_enc2),
                -dict.pairing(orb_enc1, vert_enc1),
                dict.pairing(vert_enc2, vert_enc1),
            ],
        ];

        let formula = encode_problem(&graph, &quotient_graph);
        assert_eq!(expected_formula, formula);
        Ok(())
    }

    #[test]
    fn test_descriptive_constraint() {
        let orbit_encoding = vec![(0, vec![0, 1]), (2, vec![2, 3])];
        let edge_encoding = vec![EdgeEncoding::OrbitEdge((0, 2))];
        let mut dict = SATEncodingDictionary::new();

        let orb_enc0 = dict.lookup_or_add_orbit(&0);
        let orb_enc2 = dict.lookup_or_add_orbit(&2);

        let vert_enc0 = dict.lookup_or_add_vertex(&0);
        let vert_enc1 = dict.lookup_or_add_vertex(&1);
        let vert_enc2 = dict.lookup_or_add_vertex(&2);
        let vert_enc3 = dict.lookup_or_add_vertex(&3);

        assert_eq!(1, orb_enc0);
        assert_eq!(2, orb_enc2);
        assert_eq!(3, vert_enc0);
        assert_eq!(4, vert_enc1);
        assert_eq!(5, vert_enc2);
        assert_eq!(6, vert_enc3);

        let orbit_edge = dict.pairing(orb_enc0, orb_enc2);
        let o0v0 = dict.pairing(orb_enc0, vert_enc0);
        let o0v1 = dict.pairing(orb_enc0, vert_enc1);
        let o2v2 = dict.pairing(orb_enc2, vert_enc2);
        let o2v3 = dict.pairing(orb_enc2, vert_enc3);
        let edge02 = dict.pairing(vert_enc0, vert_enc2);
        let edge03 = dict.pairing(vert_enc0, vert_enc3);
        let edge12 = dict.pairing(vert_enc1, vert_enc2);
        let edge13 = dict.pairing(vert_enc1, vert_enc3);

        let constraint02 = vec![-orbit_edge, -o0v0, -o2v2, edge02];
        let constraint03 = vec![-orbit_edge, -o0v0, -o2v3, edge03];
        let constraint12 = vec![-orbit_edge, -o0v1, -o2v2, edge12];
        let constraint13 = vec![-orbit_edge, -o0v1, -o2v3, edge13];

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
        let mut dict = SATEncodingDictionary::new();
        assert_eq!(1, dict.lookup_or_add_orbit(&0));
        assert_eq!(2, dict.lookup_or_add_vertex(&0));
        assert_eq!(3, dict.lookup_or_add_vertex(&1));
        assert_eq!(4, dict.lookup_or_add_vertex(&4));
        let pick0 = dict.pairing(1, 2);
        let pick1 = dict.pairing(1, 3);
        let pick4 = dict.pairing(1, 4);

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
    fn dict_vertex_orbits_disjunct() {
        let mut dict = SATEncodingDictionary::new();

        let vertices = vec![1, 2, 3, 5, 3, 6, 3, 5];
        let mut vertex_literals = vec![1, 2, 3, 4, 3, 5, 3, 4];
        assert_eq!(
            vertex_literals,
            vertices
                .iter()
                .map(|vert| dict.lookup_or_add_vertex(vert))
                .collect::<Vec<Literal>>()
        );

        let orbits = vec![0, 2, 5, 3, 2, 9, 0, 2];
        let mut orbit_literals = vec![6, 7, 8, 9, 7, 10, 6, 7];
        assert_eq!(
            orbit_literals,
            orbits
                .iter()
                .map(|orbit| dict.lookup_or_add_orbit(orbit))
                .collect::<Vec<Literal>>()
        );

        vertex_literals.append(&mut orbit_literals);
        let pairs = vertex_literals
            .iter()
            .combinations(2)
            .map(|pairs| dict.pairing(*pairs[0], *pairs[1]))
            .collect::<Vec<Literal>>();
        for pair_lit in pairs {
            vertex_literals.iter().for_each(|lit| {
                assert_ne!(pair_lit, *lit);
            });
        }
    }

    #[test]
    fn test_encode_graph() {
        use EdgeEncoding::VertexEdge;
        let mut graph = Graph::new_ordered(4);
        graph.add_arc(0, 1).unwrap();
        graph.add_arc(1, 2).unwrap();
        graph.add_arc(2, 3).unwrap();
        graph.add_arc(3, 1).unwrap();
        let encoded = graph.encode_high(false);
        assert_eq!(
            encoded,
            vec![
                VertexEdge((0, 1)),
                VertexEdge((1, 2)),
                VertexEdge((2, 3)),
                VertexEdge((3, 1))
            ]
        );
    }

    #[test]
    fn test_encode_orbits() {
        let orbits = vec![0, 1, 2, 0, 2, 1, 0];
        let encoded = orbits.encode_high(true);
        assert_eq!(
            encoded,
            vec![(0, vec![0, 3, 6]), (1, vec![1, 5]), (2, vec![2, 4])]
        );
    }
}
