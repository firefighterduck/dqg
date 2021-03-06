use custom_debug_derive::Debug;

use super::{Colour, GraphError, VertexIndex, DEFAULT_COLOR};

#[derive(std::fmt::Debug, PartialEq, Eq, Clone)]
pub enum GraphState {
    IndexOrdered,
    ColourGrouped,
    ColourGroupedOrdered,
    Chaos,
    Fixed,
    SparseSorted,
}

/// Fixed size graph.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Graph {
    pub vertices: Vec<Vertex>,
    size: usize,
    edge_number: usize,
    #[debug(skip)]
    pub state: GraphState,
    max_color: Colour,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vertex {
    pub index: VertexIndex,
    pub edges_to: Vec<VertexIndex>,
    pub colour: Colour,
}

impl Graph {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn number_edges(&self) -> usize {
        self.edge_number
    }

    pub fn is_sparse(&self) -> bool {
        // A complete graph has n(n-1)/2 edges for n vertices.
        // We draw the line between sparse and dense at half
        // of the possible edges in a complete graph.
        self.edge_number < self.size * (self.size - 1) / 4
    }

    #[inline]
    pub fn update_max_color(&mut self, color: Colour) {
        if color > self.max_color {
            self.max_color = color;
        }
    }

    pub fn new_ordered(n: usize) -> Self {
        let mut vertices = Vec::with_capacity(n);
        for index in 0..n {
            vertices.push(Vertex::new(index as VertexIndex, DEFAULT_COLOR));
        }
        Graph {
            vertices,
            size: n,
            edge_number: 0,
            state: GraphState::IndexOrdered,
            max_color: -1,
        }
    }

    pub fn new_with_indices(indices: &[VertexIndex], is_sorted: bool) -> Self {
        let mut vertices = Vec::with_capacity(indices.len());
        for index in indices {
            vertices.push(Vertex::new(*index, DEFAULT_COLOR));
        }
        Graph {
            vertices,
            size: indices.len(),
            edge_number: 0,
            state: if is_sorted {
                GraphState::SparseSorted
            } else {
                GraphState::Chaos
            },
            max_color: -1,
        }
    }

    pub fn set_vertex(&mut self, new_vertex: Vertex) -> Result<(), GraphError> {
        use GraphState::*;
        self.update_max_color(new_vertex.colour);
        let index = new_vertex.index;
        if self.state == IndexOrdered {
            *self
                .vertices
                .get_mut(index as usize)
                .ok_or(GraphError(index))? = new_vertex;
        } else {
            *self
                .vertices
                .iter_mut()
                .find(|vertex| vertex.index == index)
                .ok_or(GraphError(index))? = new_vertex;
            if self.state != Fixed {
                self.state = Chaos;
            }
        }
        Ok(())
    }

    pub fn get_vertex(&self, index: VertexIndex) -> Result<&Vertex, GraphError> {
        match self.state {
            GraphState::IndexOrdered => self.vertices.get(index as usize).ok_or(GraphError(index)),
            GraphState::SparseSorted => Ok(&self.vertices[self
                .vertices
                .binary_search_by(|vertex| vertex.index.cmp(&index))
                .unwrap()]),
            _ => self
                .vertices
                .iter()
                .find(|vertex| vertex.index == index)
                .ok_or(GraphError(index)),
        }
    }

    fn get_vertex_mut(&mut self, index: VertexIndex) -> Result<&mut Vertex, GraphError> {
        match self.state {
            GraphState::IndexOrdered => self
                .vertices
                .get_mut(index as usize)
                .ok_or(GraphError(index)),
            GraphState::SparseSorted => {
                let found_index = self
                    .vertices
                    .binary_search_by(|vertex| vertex.index.cmp(&index))
                    .unwrap();
                Ok(&mut self.vertices[found_index])
            }
            _ => self
                .vertices
                .iter_mut()
                .find(|vertex| vertex.index == index)
                .ok_or(GraphError(index)),
        }
    }

    pub fn add_arc(&mut self, start: VertexIndex, end: VertexIndex) -> Result<(), GraphError> {
        self.get_vertex_mut(start)?.add_edge(end);
        self.edge_number += 1;
        Ok(())
    }

    pub fn add_edge(&mut self, start: VertexIndex, end: VertexIndex) -> Result<(), GraphError> {
        self.add_arc(start, end)?;
        self.add_arc(end, start)?;
        Ok(())
    }

    pub fn lookup_edge(&self, start: &VertexIndex, end: &VertexIndex) -> bool {
        let start = *start as usize;
        debug_assert!(start < self.size);
        self.vertices[start].edges_to.binary_search(end).is_ok()
    }

    pub fn iterate_edges(&self) -> impl Iterator<Item = (VertexIndex, VertexIndex)> + '_ {
        self.vertices
            .iter()
            .flat_map(|vertex| vertex.edges_to.iter().map(move |end| (vertex.index, *end)))
    }

    /// Remove unneccessary edges.
    /// Does so by first sorting, thus trading runtime for reduced memory footprint.
    pub fn minimize(&mut self) {
        // Adjust the edge number to fit, too.
        self.edge_number = 0;
        for vertex in self.vertices.iter_mut() {
            vertex.edges_to.sort_unstable();
            vertex.edges_to.dedup();
            self.edge_number += vertex.edges_to.len();
        }
    }

    pub fn set_colours(&mut self, colours: &[Colour]) -> Result<(), GraphError> {
        for (index, colour) in colours.iter().enumerate() {
            self.get_vertex_mut(index as VertexIndex)?.colour = *colour;
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn order(&mut self, order: &[VertexIndex]) -> Result<(), GraphError> {
        let mut ordered_vertices = Vec::with_capacity(self.vertices.len());
        for index in order {
            let vertex = self.get_vertex(*index)?;
            ordered_vertices.push(vertex.clone());
        }

        self.vertices = ordered_vertices;
        self.state = GraphState::Fixed;
        Ok(())
    }

    pub fn group_colours(&mut self) {
        use GraphState::*;
        match self.state {
            IndexOrdered => {
                self.vertices.sort_by(|a, b| a.colour.cmp(&b.colour));
                self.state = ColourGroupedOrdered;
            }
            Chaos => {
                self.vertices
                    .sort_unstable_by(|a, b| a.colour.cmp(&b.colour));
                self.state = ColourGrouped;
            }
            _ => (),
        }
    }

    pub fn sort(&mut self) {
        if self.state == GraphState::SparseSorted {
            return;
        }

        if self.state != GraphState::IndexOrdered {
            self.vertices.sort_unstable();
            for vertex in self.vertices.iter_mut() {
                vertex.edges_to.sort_unstable();
            }
            self.state = GraphState::IndexOrdered;
        }
    }

    pub fn induce_subgraph(
        &self,
        remaining_vertices: &[VertexIndex],
        vertex_list_sorted: bool,
    ) -> Result<Self, GraphError> {
        let mut subgraph = Self::new_with_indices(remaining_vertices, vertex_list_sorted);

        for vertex in self.vertices.iter() {
            let include = {
                if vertex_list_sorted {
                    remaining_vertices.binary_search(&vertex.index).is_ok()
                } else {
                    remaining_vertices.contains(&vertex.index)
                }
            };

            if include {
                let new_vertex = subgraph.get_vertex_mut(vertex.index)?;
                new_vertex.colour = vertex.colour;
                let sub_edges = vertex
                    .edges_to
                    .iter()
                    .cloned()
                    .filter(|vertex| {
                        if vertex_list_sorted {
                            remaining_vertices.binary_search(vertex).is_ok()
                        } else {
                            remaining_vertices.contains(vertex)
                        }
                    })
                    .collect();
                new_vertex.edges_to = sub_edges;
            }
        }

        let sub_edge_number = subgraph.iterate_edges().count();
        subgraph.edge_number = sub_edge_number;

        Ok(subgraph)
    }

    pub fn recolor(&mut self, vertex: VertexIndex) -> Result<(), GraphError> {
        let next_color = self.max_color;
        self.max_color = next_color + 1;
        let vertex = self.get_vertex_mut(vertex)?;
        vertex.colour = next_color;
        Ok(())
    }
}

impl Vertex {
    pub fn new(index: VertexIndex, colour: Colour) -> Self {
        Vertex {
            index,
            edges_to: Vec::new(),
            colour,
        }
    }

    pub fn add_edge(&mut self, end: VertexIndex) {
        self.edges_to.push(end);
    }
}

impl PartialOrd for Vertex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.index.cmp(&other.index))
    }
}

impl Ord for Vertex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_graph_default() {
        let graph = Graph::new_ordered(120);
        assert!(graph.is_sparse());
        for (index, vertex) in graph.vertices.iter().enumerate() {
            assert_eq!(index as VertexIndex, vertex.index);
            assert_eq!(DEFAULT_COLOR, vertex.colour);
            assert!(vertex.edges_to.is_empty());
        }
    }

    #[test]
    fn test_set_vertex() {
        let mut graph = Graph::new_ordered(5);
        assert_eq!(5, graph.size());
        assert_eq!(GraphState::IndexOrdered, graph.state);

        // First with IndexOrdered

        // In bounds
        let valid_vertex = Vertex::new(2, 45);
        assert_eq!(Ok(()), graph.set_vertex(valid_vertex.clone()));

        // Negative index
        let negative_vertex = Vertex::new(-23, 9);
        assert_eq!(Err(GraphError(-23)), graph.set_vertex(negative_vertex));

        // Index out of bounds
        let oob_vertex = Vertex::new(5, 124);
        assert_eq!(Err(GraphError(5)), graph.set_vertex(oob_vertex));

        // Then with another state
        graph.state = GraphState::Chaos;

        // In bounds
        let valid_vertex_chaos = Vertex::new(3, 50);
        assert_eq!(Ok(()), graph.set_vertex(valid_vertex_chaos.clone()));

        // Negative index
        let negative_vertex_chaos = Vertex::new(-120, 9);
        assert_eq!(
            Err(GraphError(-120)),
            graph.set_vertex(negative_vertex_chaos)
        );

        // Index out of bounds
        let oob_vertex_chaos = Vertex::new(5, 124);
        assert_eq!(Err(GraphError(5)), graph.set_vertex(oob_vertex_chaos));

        assert_eq!(graph.vertices[0], Vertex::new(0, DEFAULT_COLOR));
        assert_eq!(graph.vertices[1], Vertex::new(1, DEFAULT_COLOR));
        assert_eq!(graph.vertices[2], valid_vertex);
        assert_eq!(graph.vertices[3], valid_vertex_chaos);
        assert_eq!(graph.vertices[4], Vertex::new(4, DEFAULT_COLOR));
    }

    #[test]
    fn test_get_vertex() -> Result<(), GraphError> {
        let mut graph = Graph::new_ordered(5);
        assert_eq!(5, graph.size());
        assert_eq!(GraphState::IndexOrdered, graph.state);

        // First with IndexOrdered

        // In bounds
        let valid_result = graph.get_vertex_mut(2)?;
        assert_eq!(&mut Vertex::new(2, DEFAULT_COLOR), valid_result);

        // Negative index
        assert_eq!(Err(GraphError(-3)), graph.get_vertex_mut(-3));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));

        // Then with another state
        graph.state = GraphState::Chaos;

        // In bounds
        let valid_result = graph.get_vertex_mut(3)?;
        assert_eq!(&mut Vertex::new(3, DEFAULT_COLOR), valid_result);

        // Negative index
        assert_eq!(Err(GraphError(-1)), graph.get_vertex_mut(-1));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));

        Ok(())
    }

    #[test]
    fn test_induce_subgraph() -> Result<(), GraphError> {
        let mut graph = Graph::new_ordered(5);
        for i in 0..4 {
            graph.add_edge(i, i + 1)?;
        }

        let vertex_subset = vec![1, 3, 4];

        let mut expected_subgraph = Graph::new_with_indices(&vertex_subset, true);
        expected_subgraph.add_edge(3, 4)?;

        assert_eq!(
            expected_subgraph,
            graph.induce_subgraph(&vertex_subset, true)?
        );

        Ok(())
    }
}
