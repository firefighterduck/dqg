//! This file contains the high and low level encodings
//! needed to encode the descriptive quotient problem
//! as a CNF formula which can then be decided by a SAT solver.

use std::{collections::HashMap, os::raw::c_int};

use itertools::Itertools;
use kissat_rs::Literal;

use crate::{
    graph::{Graph, VertexIndex},
    quotient::{Orbits, QuotientGraph},
};

/// Trait that defines whether a type can be encoded
/// into a high level view of a SAT formula.
trait HighLevelEncoding {
    type HighLevelRepresentation;
    fn encode_high(&self) -> Self::HighLevelRepresentation;
}

/// Trait that defines a dictionary/oracle that
/// uniquely determines the literal for a high level
/// representation of a SAT formula part.
trait EncodingDictionary<HighLevelEncoding> {
    fn lookup(&mut self, element: &HighLevelEncoding) -> Option<Literal>;

    fn lookup_or_add(&mut self, element: &HighLevelEncoding) -> Literal;
}

trait SATEncoding {
    fn encode_sat<Dict>(&self, dict: &Dict) -> Vec<Vec<Literal>>
    where
        Dict: EncodingDictionary<Self>,
        Self: Sized;
}

pub type EdgeEncoding = (VertexIndex, VertexIndex);

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

pub type OrbitsEncoding = Vec<Vec<VertexIndex>>;

impl HighLevelEncoding for Orbits {
    type HighLevelRepresentation = OrbitsEncoding;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        self.iter()
            .enumerate()
            .sorted_by(|(_, orbit_a), (_, orbit_b)| orbit_a.cmp(orbit_b))
            .group_by(|(_, orbit)| **orbit)
            .into_iter()
            .map(|(_, vertices)| {
                vertices
                    .into_iter()
                    .map(|(vertex, _)| vertex as VertexIndex)
                    .collect()
            })
            .collect()
    }
}

pub type QuotientGraphEncoding = (Vec<EdgeEncoding>, OrbitsEncoding);

impl HighLevelEncoding for QuotientGraph {
    type HighLevelRepresentation = QuotientGraphEncoding;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        (self.quotient_graph.encode_high(), self.orbits.encode_high())
    }
}

pub struct SATEncodingDictionary {
    literal_counter: Literal,
    vertex_map: HashMap<VertexIndex, Literal>,
    orbit_map: HashMap<Vec<VertexIndex>, Literal>,
}

impl SATEncodingDictionary {
    fn new() -> Self {
        SATEncodingDictionary {
            literal_counter: 0,
            vertex_map: HashMap::new(),
            orbit_map: HashMap::new(),
        }
    }

    fn lookup_vertex(&self, vertex: &VertexIndex) -> Option<Literal> {
        self.vertex_map.get(vertex).map(|literal_ref| *literal_ref)
    }

    fn get_new_literal(&mut self) -> Literal {
        let new_literal = self.literal_counter;
        self.literal_counter += 1;
        new_literal
    }

    fn lookup_vertex_or_add(&mut self, vertex: &VertexIndex) -> Literal {
        if let Some(literal) = self.vertex_map.get(vertex) {
            *literal
        } else {
            let new_lit = self.get_new_literal();
            self.vertex_map.insert(*vertex, new_lit);
            new_lit
        }
    }

    fn pairing(&mut self, first: VertexIndex, second: VertexIndex) -> Literal {
        todo!()
    }
}

impl EncodingDictionary<EdgeEncoding> for SATEncodingDictionary {
    fn lookup(&mut self, element: &EdgeEncoding) -> Option<Literal> {
        let (start, end) = element;
        let start_lit = self.lookup_vertex(start);
        let end_lit = self.lookup_vertex(end);
        start_lit
            .zip(end_lit)
            .map(|(start, end)| self.pairing(start, end))
    }

    fn lookup_or_add(&mut self, element: &EdgeEncoding) -> Literal {
        if let Some(literal) = self.lookup(element) {
            literal
        } else {
            let (start, end) = element;
            let start_lit = self.lookup_vertex_or_add(start);
            let end_lit = self.lookup_vertex_or_add(end);
            self.pairing(start_lit, end_lit)
        }
    }
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
        assert_eq!(encoded, vec![vec![0, 3, 6], vec![1, 5], vec![2, 4]]);
    }
}
