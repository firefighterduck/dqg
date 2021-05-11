//! Representation of graphs as well as
//! functionalities to build them from
//! simple building blocks or switch to
//! to a representation understand by nauty.
use custom_debug_derive::Debug;
use nauty_Traces_sys::{empty_graph, ADDONEARC, SETWORDSNEEDED};
use std::os::raw::c_int;

pub type Colour = c_int;
pub type VertexIndex = c_int;

#[derive(Debug, PartialEq, Eq)]
pub struct GraphError(VertexIndex);

#[derive(Debug, PartialEq, Eq)]
enum GraphState {
    IndexOrdered,
    ColourGrouped,
    ColourGroupedOrdered,
    Chaos,
    Fixed,
}

/// Fixed size graph.
#[derive(Debug, PartialEq, Eq)]
pub struct Graph {
    vertices: Vec<Vertex>,
    size: usize,
    #[debug(skip)]
    state: GraphState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vertex {
    pub index: VertexIndex,
    pub edges_to: Vec<VertexIndex>,
    pub colour: Colour,
}

#[allow(clippy::ptr_arg)]
#[cfg(not(tarpaulin_include))]
pub fn bin_fmt(vec: &Vec<u64>, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{")?;
    for number in vec {
        write!(f, "{:#066b}", number)?;
    }
    write!(f, "}}")?;

    Ok(())
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

impl Graph {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn new_ordered(n: usize) -> Self {
        let mut vertices = Vec::with_capacity(n);
        for index in 0..n {
            vertices.push(Vertex::new(index as VertexIndex, -1));
        }
        Graph {
            vertices,
            size: n,
            state: GraphState::IndexOrdered,
        }
    }

    pub fn new_with_indices(indices: &[VertexIndex]) -> Self {
        let mut vertices = Vec::with_capacity(indices.len());
        for index in indices {
            vertices.push(Vertex::new(*index, -1));
        }
        Graph {
            vertices,
            size: indices.len(),
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
        Ok(())
    }

    pub fn add_edge(&mut self, start: VertexIndex, end: VertexIndex) -> Result<(), GraphError> {
        self.add_arc(start, end)?;
        self.add_arc(end, start)
    }

    pub fn iterate_edges<F>(&self, mut f: F)
    where
        F: FnMut((VertexIndex, VertexIndex)),
    {
        for vertex in self.vertices.iter() {
            for end in vertex.edges_to.iter() {
                f((vertex.index, *end));
            }
        }
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

    pub fn prepare_nauty(&mut self) -> NautyGraph {
        let n = self.vertices.len();
        let m = SETWORDSNEEDED(n);

        let mut nauty_graph = NautyGraph {
            adjacency_matrix: empty_graph(m, n),
            vertex_order: Vec::with_capacity(n),
            partition: Vec::with_capacity(n),
        };

        if self.state != GraphState::Fixed {
            self.sort();
            self.group_colours();
        }

        for vertex in self.vertices.iter() {
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

        let mut last_colour = -2; // -1 is standard for not set and other negative numbers can not arise.
        for colour in nauty_graph.partition.iter_mut().rev() {
            if *colour != last_colour {
                last_colour = *colour;
                *colour = 0;
            } else {
                *colour = 1;
            }
        }

        nauty_graph
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

impl NautyGraph {
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

#[cfg(test)]
mod test {
    use nauty_Traces_sys::{densenauty, optionblk, statsblk, FALSE};

    use super::*;

    #[test]
    fn new_graph_default() {
        let graph = Graph::new_ordered(120);
        for (index, vertex) in graph.vertices.iter().enumerate() {
            assert_eq!(index as VertexIndex, vertex.index);
            assert_eq!(-1, vertex.colour);
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

        assert_eq!(graph.vertices[0], Vertex::new(0, -1));
        assert_eq!(graph.vertices[1], Vertex::new(1, -1));
        assert_eq!(graph.vertices[2], valid_vertex);
        assert_eq!(graph.vertices[3], valid_vertex_chaos);
        assert_eq!(graph.vertices[4], Vertex::new(4, -1));
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
        assert_eq!(&mut Vertex::new(2, -1), valid_result.unwrap());

        // Negative index
        assert_eq!(Err(GraphError(-3)), graph.get_vertex_mut(-3));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));

        // Then with another state
        graph.state = GraphState::Chaos;

        // In bounds
        let valid_result = graph.get_vertex_mut(3);
        assert!(valid_result.is_ok());
        assert_eq!(&mut Vertex::new(3, -1), valid_result.unwrap());

        // Negative index
        assert_eq!(Err(GraphError(-1)), graph.get_vertex_mut(-1));

        // Index out of bounds
        assert_eq!(Err(GraphError(5)), graph.get_vertex_mut(5));
    }

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

        let mut nauty_graph = graph.prepare_nauty();
        println!("{:?}", graph.state);
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

        let mut nauty_graph = graph.prepare_nauty();
        println!("{:?}", graph.state);
        assert_eq!(nauty_graph.vertex_order, [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(nauty_graph.partition, [1, 1, 1, 1, 1, 1, 1, 0]);
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

        assert_eq!(orbits, [0, 0, 0, 0, 0, 0, 0, 0]);
        Ok(())
    }
}
