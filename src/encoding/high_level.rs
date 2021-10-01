use itertools::Itertools;

use crate::{
    graph::{Graph, VertexIndex},
    quotient::{Orbits, QuotientGraph},
};

/// Trait that defines whether a type can be encoded
/// into a high level view of a SAT formula.
pub trait HighLevelEncoding {
    type HighLevelRepresentation;
    fn encode_high(&self) -> Self::HighLevelRepresentation;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct EdgeEncoding(pub VertexIndex, pub VertexIndex);

impl EdgeEncoding {
    pub fn get_edge(&self) -> (VertexIndex, VertexIndex) {
        (self.0, self.1)
    }
}

impl HighLevelEncoding for Graph {
    type HighLevelRepresentation = Vec<EdgeEncoding>;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        self.iterate_edges()
            .map(|(start, end)| EdgeEncoding(start, end))
            .collect()
    }
}

pub type OrbitEncoding = (VertexIndex, Vec<VertexIndex>);

impl HighLevelEncoding for Orbits {
    type HighLevelRepresentation = Vec<OrbitEncoding>;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        self.iter()
            .enumerate()
            .filter(|(_, orbit)| *orbit >= &0)
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

#[derive(Debug, Clone)]
pub struct QuotientGraphEncoding(pub Vec<EdgeEncoding>, pub Vec<OrbitEncoding>);

impl HighLevelEncoding for QuotientGraph {
    type HighLevelRepresentation = QuotientGraphEncoding;

    fn encode_high(&self) -> Self::HighLevelRepresentation {
        QuotientGraphEncoding(self.quotient_graph.encode_high(), self.orbits.encode_high())
    }
}
