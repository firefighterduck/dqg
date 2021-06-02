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

    fn get_new_literal(&mut self) -> Literal {
        let new_literal = self.literal_counter;

        // Kissat doesn't allow variables over 2^28-1.
        debug_assert!(new_literal < MAX_LITERAL);

        self.literal_counter += 1;
        new_literal
    }
}
