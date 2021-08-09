use crate::{
    encoding::QuotientGraphEncoding,
    graph::{Graph, VertexIndex},
};

pub fn is_transversal_consistent(
    transversal: &[(VertexIndex, VertexIndex)],
    graph: &Graph,
    quotient: QuotientGraphEncoding,
) -> bool {
    for edge in quotient.0.iter() {
        let start = transversal[transversal
            .binary_search_by(|(orbit, _)| orbit.cmp(&edge.0))
            .expect("Transversal didn't contain orbit!")]
        .1;
        let end = transversal[transversal
            .binary_search_by(|(orbit, _)| orbit.cmp(&edge.1))
            .expect("Transversal didn't contain orbit!")]
        .1;

        if !graph.lookup_edge(&start, &end) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{encoding::EdgeEncoding, Error};

    #[test]
    fn test_is_transversal_consistent_true() -> Result<(), Error> {
        let mut graph = Graph::new_ordered(8);
        graph.add_edge(0, 1)?;
        graph.add_edge(0, 3)?;
        graph.add_edge(0, 4)?;
        graph.add_edge(1, 2)?;
        graph.add_edge(1, 5)?;
        graph.add_edge(2, 3)?;
        graph.add_edge(2, 6)?;
        graph.add_edge(3, 7)?;
        graph.add_edge(4, 5)?;
        graph.add_edge(4, 7)?;
        graph.add_edge(5, 6)?;
        graph.add_edge(6, 7)?;

        let quotient1: QuotientGraphEncoding = QuotientGraphEncoding(
            vec![EdgeEncoding(0, 2)],
            vec![(0, vec![0, 1, 4, 5]), (2, vec![2, 3, 6, 7])],
        );
        let transversal1_1 = vec![(0, 0), (2, 3)];
        assert!(is_transversal_consistent(
            &transversal1_1,
            &graph,
            quotient1.clone()
        ));
        let transversal1_2 = vec![(0, 5), (2, 6)];
        assert!(is_transversal_consistent(
            &transversal1_2,
            &graph,
            quotient1.clone()
        ));
        let transversal1_3 = vec![(0, 0), (2, 6)];
        assert!(!is_transversal_consistent(
            &transversal1_3,
            &graph,
            quotient1.clone()
        ));

        let quotient2: QuotientGraphEncoding = QuotientGraphEncoding(
            vec![EdgeEncoding(0, 1), EdgeEncoding(0, 4), EdgeEncoding(1, 2)],
            vec![
                (0, vec![0, 5, 7]),
                (1, vec![1, 3, 6]),
                (2, vec![2]),
                (4, vec![4]),
            ],
        );
        let transversal2_1 = vec![(0, 0), (1, 1), (2, 2), (4, 4)];
        assert!(is_transversal_consistent(
            &transversal2_1,
            &graph,
            quotient2.clone()
        ));
        let transversal2_2 = vec![(0, 5), (1, 6), (2, 2), (4, 4)];
        assert!(is_transversal_consistent(
            &transversal2_2,
            &graph,
            quotient2.clone()
        ));
        let transversal2_3 = vec![(0, 0), (1, 6), (2, 2), (4, 4)];
        assert!(!is_transversal_consistent(
            &transversal2_3,
            &graph,
            quotient2.clone()
        ));

        Ok(())
    }

    #[test]
    fn test_is_transversal_consistent_false() -> Result<(), Error> {
        let mut graph = Graph::new_ordered(8);
        graph.add_edge(0, 1)?;
        graph.add_edge(0, 4)?;
        graph.add_edge(1, 7)?;
        graph.add_edge(2, 3)?;
        graph.add_edge(2, 6)?;
        graph.add_edge(3, 6)?;
        graph.add_edge(4, 5)?;
        graph.add_edge(6, 7)?;

        let quotient: QuotientGraphEncoding = QuotientGraphEncoding(
            vec![
                EdgeEncoding(0, 1),
                EdgeEncoding(0, 4),
                EdgeEncoding(1, 5),
                EdgeEncoding(4, 5),
            ],
            vec![
                (0, vec![0, 2]),
                (1, vec![1, 3]),
                (4, vec![4, 6]),
                (5, vec![5, 7]),
            ],
        );

        for pick0 in [0, 2] {
            for pick1 in [1, 3] {
                for pick4 in [4, 6] {
                    for pick5 in [5, 7] {
                        let transversal = vec![(0, pick0), (1, pick1), (4, pick4), (5, pick5)];
                        assert!(!is_transversal_consistent(
                            &transversal,
                            &graph,
                            quotient.clone()
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
