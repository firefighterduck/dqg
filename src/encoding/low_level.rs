use crate::graph::Graph;

use super::{
    encoding_dict::SATEncodingDictionary,
    high_level::{EdgeEncoding, OrbitEncoding, QuotientGraphEncoding},
    Formula,
};

pub trait SATEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary, original_graph: &Graph) -> Formula;
}

impl SATEncoding for OrbitEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary, _original_graph: &Graph) -> Formula {
        // This is actually the encoding that a valid transversal
        // can only choose one element from the orbit.

        // Encode the EO problem
        // Possible encodings:
        // - pairwise: (x1 || x2 || ... || xn) && for all i,j (~xi || ~xj), size = (n^2-n)/2
        // - bitwise: with aux vars, size = n*ceil(ld n), ceil(ld n) aux vars
        // - ladder: however this works, 3(n-1) binary clauses, n-1 ternary clauses, n-1 aux vars
        // - matrix: how the heck does this even, 2*sqrt(n) aux vars, 1 n-ary clause, 1 sqrt(n)-ary clause, 1 n/sqrt(n)-ary clause, 2n+4*sqrt(n)+O(fourth root n) binary clauses

        // For now we use pairwise encoding, because it's easy to implement
        // Disjunction of all vertex-in-orbit pairs to encode AT LEAST ONE
        // ---------------------------------------------------------------
        // \/ vi for all vi in the orbit
        let (orbit, orbit_elements) = self;
        let mut orbit_element_encodings = Vec::with_capacity(orbit_elements.len());

        for orbit_element in orbit_elements {
            orbit_element_encodings.push(dict.lookup_pairing(*orbit, *orbit_element));
        }

        vec![orbit_element_encodings]
    }
}

impl SATEncoding for QuotientGraphEncoding {
    fn encode_sat(&self, dict: &mut SATEncodingDictionary, original_graph: &Graph) -> Formula {
        // This is actually the encoding that edges between two
        // vertices (i.e. two orbits) of a quotient graph is preserved
        // when the transversal chooses two vertices from the orbits.
        let QuotientGraphEncoding(quotient_edges, orbits) = self;
        let mut formula = Vec::new();

        // for all (o1,o2) edges in the quotient graph G\O (i.e. o1, o2 in O)
        for (start_orbit, end_orbit) in quotient_edges.iter().map(EdgeEncoding::get_edge) {
            let start_orbit_elements = {
                let index = orbits.binary_search_by(|(orbit,_)| orbit.cmp(&start_orbit)) .expect(
                    "The edges were computed from the orbits, how can there be no fitting orbit?",
                );
                &orbits[index].1
            };
            let end_orbit_elements =
                {
                    let index = orbits.binary_search_by(|(orbit,_)| orbit.cmp(&end_orbit)) .expect(
                    "The edges were computed from the orbits, how can there be no fitting orbit?",
                );
                    &orbits[index].1
                };

            // for all vertices v1 in o1
            for start_orbit_element in start_orbit_elements {
                // for all vertices v2 in o2
                'end: for end_orbit_element in end_orbit_elements {
                    // If the edge (v1,v2) for the two picked vertices exists
                    // in the original graph, we do not need to encode it.
                    if original_graph.lookup_edge(start_orbit_element, end_orbit_element) {
                        continue 'end;
                    }

                    let start_orbit_relation =
                        dict.lookup_pairing(start_orbit, *start_orbit_element);
                    let end_orbit_relation = dict.lookup_pairing(end_orbit, *end_orbit_element);

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
