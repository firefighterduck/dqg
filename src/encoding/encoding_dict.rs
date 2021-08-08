use std::collections::HashMap;

use custom_debug_derive::Debug;
use kissat_rs::Literal;

use crate::graph::VertexIndex;

const MAX_LITERAL: Literal = 2i32.pow(28) - 1;

#[derive(Debug)]
pub struct SATEncodingDictionary {
    literal_counter: Literal,
    #[debug(skip)]
    literal_map: HashMap<i64, Literal>,
}

impl Default for SATEncodingDictionary {
    fn default() -> Self {
        SATEncodingDictionary {
            literal_counter: 1,
            literal_map: HashMap::new(),
        }
    }
}

impl SATEncodingDictionary {
    /// Lookup the literal to which an orbit/vertex pair is mapped.
    pub fn lookup_pairing(&mut self, orbit: Literal, vertex: Literal) -> Literal {
        let pairing_result = Self::pairing(orbit, vertex);

        if let Some(literal) = self.literal_map.get(&pairing_result) {
            *literal
        } else {
            let literal = self.get_new_literal();
            self.literal_map.insert(pairing_result, literal);
            literal
        }
    }

    fn pairing(orbit: VertexIndex, vertex: VertexIndex) -> i64 {
        let orbit_part = (orbit as i64) << 32;
        orbit_part + (vertex as i64)
    }

    fn unpair(pairing: i64) -> (VertexIndex, VertexIndex) {
        let orbit = pairing >> 32;
        let vertex = pairing as i32;
        (orbit as i32, vertex)
    }

    fn get_new_literal(&mut self) -> Literal {
        let new_literal = self.literal_counter;

        // Kissat doesn't allow variables over 2^28-1.
        debug_assert!(new_literal < MAX_LITERAL);

        self.literal_counter += 1;
        new_literal
    }

    pub fn destroy(mut self) -> Vec<(VertexIndex, VertexIndex)> {
        let mut pairs = vec![(-1, -1); self.literal_counter as usize];
        for (pairing, literal) in self.literal_map.drain() {
            pairs[literal as usize] = Self::unpair(pairing);
        }
        pairs
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_pair() {
        let orbit = 0x12345678;
        let vertex = 0x07654321;
        let pair = SATEncodingDictionary::pairing(orbit, vertex);
        assert_eq!(0x1234567807654321, pair);
    }

    #[test]
    fn test_unpair() {
        let (orbit, vertex) = SATEncodingDictionary::unpair(0x1234567801234567);
        assert_eq!(0x12345678, orbit);
        assert_eq!(0x01234567, vertex);
    }
}
