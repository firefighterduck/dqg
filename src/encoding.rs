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

/// Trait that defines whether a type can be encoded
/// into a high level view of a SAT formula.
trait HighLevelEncoding {
    type HighLevelRepresentation;
    fn encode_high(&self) -> Self::HighLevelRepresentation;
}

trait SATEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary) -> Formula;
}

type EdgeEncoding = (VertexIndex, VertexIndex);

impl HighLevelEncoding for Graph {
    type HighLevelRepresentation = Vec<EdgeEncoding>;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        let mut edges = Vec::new();

        self.iterate_edges(|edge| {
            edges.push(edge);
        });

        edges
    }
}

type OrbitEncoding = (VertexIndex, Vec<VertexIndex>);

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

    pub fn lookup_edge(&mut self, edge: &EdgeEncoding) -> Option<Literal> {
        let (start, end) = edge;
        let start_lit = self.lookup_vertex(start);
        let end_lit = self.lookup_vertex(end);
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
            let (start, end) = edge;
            let start_lit = self.lookup_or_add_vertex(start);
            let end_lit = self.lookup_or_add_vertex(end);
            self.pairing(start_lit, end_lit)
        }
    }

    /// This computes the Cantor pairing function for th two given literals.
    fn pairing(&mut self, first: Literal, second: Literal) -> Literal {
        let pairing_result = (first + second) * (first + second + 1) / 2 + second;
        // The return value must also be a valid literal.
        // Whereas the normally assigned literals grow from 0,
        // the paired ones grow from the positive max value to reduce collisions.
        assert!(
            self.literal_counter < Literal::MAX - pairing_result,
            "SAT vertex variable space and pair variable space intersect!"
        );

        Literal::MAX - pairing_result
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
        for orbit_element1 in orbit_element_encodings.iter() {
            for orbit_element2 in orbit_element_encodings.iter() {
                if orbit_element1 != orbit_element2 {
                    // -v1 || -v2; v1!=v2; v1, v2 in the given orbit
                    formula.push(vec![-orbit_element1, -orbit_element2]);
                }
            }
        }

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
        for (start_orbit, end_orbit) in quotient_edges {
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
        .encode_high()
        .into_iter()
        .flat_map(|edge| edge.encode_sat(&mut dict))
        .collect();
    let (quotient_edges, orbits) = quotient_graph.encode_high();
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
    use super::*;

    #[test]
    fn test_encode_graph() {
        let mut graph = Graph::new_ordered(4);
        graph.add_arc(0, 1);
        graph.add_arc(1, 2);
        graph.add_arc(2, 3);
        graph.add_arc(3, 1);
        let encoded = graph.encode_high();
        assert_eq!(encoded, vec![(0, 1), (1, 2), (2, 3), (3, 1)]);
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
