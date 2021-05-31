use custom_debug_derive::Debug;

use super::{Colour, GraphError, VertexIndex, DEFAULT_COLOR};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GraphState {
    IndexOrdered,
    ColourGrouped,
    ColourGroupedOrdered,
    Chaos,
    Fixed,
}

/// Fixed size graph.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Graph {
    pub vertices: Vec<Vertex>,
    size: usize,
    edge_number: usize,
    #[debug(skip)]
    pub state: GraphState,
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
        }
    }

    pub fn new_with_indices(indices: &[VertexIndex]) -> Self {
        let mut vertices = Vec::with_capacity(indices.len());
        for index in indices {
            vertices.push(Vertex::new(*index, DEFAULT_COLOR));
        }
        Graph {
            vertices,
            size: indices.len(),
            edge_number: 0,
            state: GraphState::Chaos,
        }
    }

    pub fn set_vertex(&mut self, new_vertex: Vertex) -> Result<(), GraphError> {
        use GraphState::*;
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

    #[cfg(test)]
    pub fn get_vertex(&mut self, index: VertexIndex) -> Result<&Vertex, GraphError> {
        match self.state {
            GraphState::IndexOrdered => self.vertices.get(index as usize).ok_or(GraphError(index)),
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
        self.edge_number += 1;
        self.add_arc(end, start)?;
        self.edge_number += 1;
        Ok(())
    }

    pub fn lookup_edge(&self, start: &VertexIndex, end: &VertexIndex) -> bool {
        let start = *start as usize;
        assert!(start < self.size);
        self.vertices[start].edges_to.iter().any(|edge| edge == end)
    }

    pub fn iterate_edges(&self) -> impl Iterator<Item = (VertexIndex, VertexIndex)> + '_ {
        self.vertices
            .iter()
            .flat_map(|vertex| vertex.edges_to.iter().map(move |end| (vertex.index, *end)))
    }

    /// Remove unneccessary edges.
    /// Does so by first sorting, thus trading runtime for reduced memory footprint.
    pub fn minimize(&mut self) {
        for vertex in self.vertices.iter_mut() {
            vertex.edges_to.sort_unstable();
            vertex.edges_to.dedup();
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
        if self.state != GraphState::IndexOrdered {
            self.vertices.sort_unstable_by(|a, b| a.index.cmp(&b.index));
            self.state = GraphState::IndexOrdered;
        }
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
    fn test_get_vertex() {
        let mut graph = Graph::new_ordered(5);
        assert_eq!(5, graph.size());
        assert_eq!(GraphState::IndexOrdered, graph.state);

        // First with IndexOrdered

        // In bounds
        let valid_result = graph.get_vertex_mut(2);
        assert!(valid_result.is_ok());
        assert_eq!(&mut Vertex::new(2, DEFAULT_COLOR), valid_result.unwrap());

        // Negative index
        assert_eq!(Err(GraphError(-3)), graph.get_vertex_mut(-3));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));

        // Then with another state
        graph.state = GraphState::Chaos;

        // In bounds
        let valid_result = graph.get_vertex_mut(3);
        assert!(valid_result.is_ok());
        assert_eq!(&mut Vertex::new(3, DEFAULT_COLOR), valid_result.unwrap());

        // Negative index
        assert_eq!(Err(GraphError(-1)), graph.get_vertex_mut(-1));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));
    }
}
