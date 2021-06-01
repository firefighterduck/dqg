use custom_debug_derive::Debug;
use itertools::Itertools;
use nauty_Traces_sys::{empty_graph, SparseGraph, ADDONEARC, SETWORDSNEEDED};
use std::{convert::TryInto, os::raw::c_int};

use super::{Colour, Graph, GraphState, VertexIndex};
use crate::debug::bin_fmt;

fn encode_colours(partition: &mut [Colour]) {
    let mut last_colour = c_int::MIN; // Negative numbers should not arise or if they do, they should be bigger than this.
    for colour in partition.iter_mut().rev() {
        if *colour != last_colour {
            last_colour = *colour;
            *colour = 0;
        } else {
            *colour = 1;
        }
    }
}

#[derive(Debug)]
pub struct NautyGraph {
    /// actual graph
    #[debug(with = "bin_fmt")]
    pub adjacency_matrix: Vec<u64>,
    /// lab
    pub vertex_order: Vec<VertexIndex>,
    /// ptn aka the colouring
    pub partition: Vec<VertexIndex>,
}

#[derive(Debug)]
pub struct TracesGraph {
    /// actual graph
    pub sparse_graph: SparseGraph,
    /// lab
    pub vertex_order: Vec<VertexIndex>,
    /// ptn aka the colouring
    pub partition: Vec<VertexIndex>,
}

pub type SparseNautyGraph = TracesGraph;

impl NautyGraph {
    pub fn from_graph(graph: &mut Graph) -> NautyGraph {
        let n = graph.size();
        let m = SETWORDSNEEDED(n);

        let mut nauty_graph = NautyGraph {
            adjacency_matrix: empty_graph(m, n),
            vertex_order: Vec::with_capacity(n),
            partition: Vec::with_capacity(n),
        };

        if graph.state != GraphState::Fixed {
            graph.sort();
            graph.group_colours();
        }

        for vertex in graph.vertices.iter() {
            nauty_graph.vertex_order.push(vertex.index);

            nauty_graph.partition.push(vertex.colour);

            for end in vertex.edges_to.iter() {
                ADDONEARC(
                    &mut nauty_graph.adjacency_matrix,
                    vertex.index as usize,
                    *end as usize,
                    m,
                );
            }
        }

        encode_colours(&mut nauty_graph.partition);

        nauty_graph
    }

    pub fn check_valid(&self) -> bool {
        let n = self.partition.len();
        let m = SETWORDSNEEDED(n);

        self.adjacency_matrix.len() == n * m && self.vertex_order.len() == n
    }

    pub fn graph_repr_sizes(&self) -> (usize, usize) {
        let n = self.partition.len();
        (n, SETWORDSNEEDED(n))
    }
}

impl TracesGraph {
    pub fn from_graph(graph: &mut Graph) -> TracesGraph {
        let number_vertices = graph.size();
        let number_edges = graph.number_edges();

        let mut traces_graph = TracesGraph {
            sparse_graph: SparseGraph::new(number_vertices, number_edges),
            vertex_order: Vec::with_capacity(number_vertices),
            partition: Vec::with_capacity(number_vertices),
        };

        if graph.state != GraphState::Fixed {
            graph.sort();
            graph.group_colours();
        }

        // Encode order and colors
        for vertex in graph.vertices.iter() {
            traces_graph.vertex_order.push(vertex.index);
            traces_graph.partition.push(vertex.colour);
        }
        encode_colours(&mut traces_graph.partition);

        // Encode graph. Vertices must be ordered with increasing indices.
        let mut edge_counter = 0usize;
        for (index, vertex) in graph
            .vertices
            .iter()
            .sorted_by(|a, b| a.index.cmp(&b.index))
            .enumerate()
        {
            debug_assert_eq!(index as i32, vertex.index);
            traces_graph.sparse_graph.d[index] = vertex.edges_to.len().try_into().unwrap();
            traces_graph.sparse_graph.v[index] = edge_counter.try_into().unwrap();

            for end in vertex.edges_to.iter() {
                traces_graph.sparse_graph.e[edge_counter] = *end;
                edge_counter += 1;
            }
        }

        traces_graph
    }
}

#[cfg(test)]
mod test {
    use nauty_Traces_sys::{
        densenauty, optionblk, statsblk, Traces, TracesOptions, TracesStats, FALSE,
    };

    use crate::graph::GraphError;

    use super::*;

    #[test]
    fn correct_nauty_repr() -> Result<(), GraphError> {
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

        let order = [2, 0, 1, 3, 4, 5, 6, 7];
        let colours = [2, 2, 1, 2, 2, 2, 2, 2];
        graph.set_colours(&colours)?;
        graph.order(&order)?;

        let mut nauty_graph = NautyGraph::from_graph(&mut graph);
        assert_eq!(nauty_graph.vertex_order, order);
        assert_eq!(nauty_graph.partition, [0, 1, 1, 1, 1, 1, 1, 0]);
        assert!(nauty_graph.check_valid());
        assert_eq!(nauty_graph.graph_repr_sizes(), (8, 1));

        let mut options = optionblk::default();
        options.writeautoms = FALSE;
        options.defaultptn = FALSE;
        let mut stats = statsblk::default();
        let mut orbits = vec![0; 8];

        unsafe {
            densenauty(
                nauty_graph.adjacency_matrix.as_mut_ptr(),
                nauty_graph.vertex_order.as_mut_ptr(),
                nauty_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                1,
                8,
                std::ptr::null_mut(),
            );
        }

        assert_eq!(orbits, [0, 1, 2, 1, 4, 0, 1, 0]);
        Ok(())
    }

    #[test]
    fn correct_nauty_repr2() -> Result<(), GraphError> {
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

        let mut nauty_graph = NautyGraph::from_graph(&mut graph);
        assert_eq!(nauty_graph.vertex_order, [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(nauty_graph.partition, [1, 1, 1, 1, 1, 1, 1, 0]);
        assert!(nauty_graph.check_valid());
        assert_eq!(nauty_graph.graph_repr_sizes(), (8, 1));

        let mut options = optionblk::default();
        options.writeautoms = FALSE;
        let mut stats = statsblk::default();
        let mut orbits = vec![0; 8];

        unsafe {
            densenauty(
                nauty_graph.adjacency_matrix.as_mut_ptr(),
                nauty_graph.vertex_order.as_mut_ptr(),
                nauty_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                1,
                8,
                std::ptr::null_mut(),
            );
        }

        assert_eq!(orbits, [0, 0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }

    #[test]
    fn correct_traces_repr() -> Result<(), GraphError> {
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

        let order = [2, 1, 0, 3, 4, 5, 6, 7];
        let colours = [2, 2, 1, 2, 2, 2, 2, 2];
        graph.set_colours(&colours)?;
        graph.order(&order)?;

        let mut traces_graph = TracesGraph::from_graph(&mut graph);
        assert_eq!(traces_graph.vertex_order, order);
        assert_eq!(traces_graph.partition, [0, 1, 1, 1, 1, 1, 1, 0]);

        let mut options = TracesOptions::default();
        options.defaultptn = FALSE;
        options.digraph = FALSE;
        options.getcanon = FALSE;
        let mut stats = TracesStats::default();
        let mut orbits = vec![0; 8];

        unsafe {
            Traces(
                &mut (&mut traces_graph.sparse_graph).into(),
                traces_graph.vertex_order.as_mut_ptr(),
                traces_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                std::ptr::null_mut(),
            );
        }

        assert_eq!(orbits, [0, 1, 2, 1, 4, 0, 1, 0]);
        Ok(())
    }
}
