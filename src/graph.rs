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
enum GraphState {
    IndexOrdered,
    ColourGrouped,
    ColourGroupedOrdered,
    Chaos,
}

#[derive(Debug)]
pub struct Graph {
    vertices: Vec<Vertex>,
    #[debug(skip)]
    state: GraphState,
    #[debug(skip)]
    keep_state_auto: bool,
}

#[derive(Debug, Clone)]
pub struct Vertex {
    pub index: VertexIndex,
    pub edges_to: Vec<VertexIndex>,
    pub colour: Colour,
}

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
    pub fn new_empty(n: usize) -> Self {
        Graph {
            vertices: Vec::with_capacity(n),
            state: GraphState::Chaos,
            keep_state_auto: true,
        }
    }

    pub fn size(&self) -> usize {
        self.vertices.len()
    }

    pub fn new_ordered(n: usize) -> Self {
        let mut vertices = Vec::with_capacity(n);
        for index in 0..n {
            vertices.push(Vertex::new(index as VertexIndex, 0));
        }
        Graph {
            vertices,
            state: GraphState::IndexOrdered,
            keep_state_auto: true,
        }
    }

    pub fn from_colour_list(coloured_vertices: &[Colour]) -> Self {
        let mut vertices = Vec::with_capacity(coloured_vertices.len());
        for (index, colour) in coloured_vertices.iter().enumerate() {
            vertices.push(Vertex::new(index as VertexIndex, *colour));
        }
        Graph {
            vertices,
            state: GraphState::Chaos,
            keep_state_auto: true,
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        use GraphState::*;
        match self.state {
            IndexOrdered => {
                let index = vertex.index as usize;
                if let Some(old_vertex) = self.vertices.get_mut(index) {
                    *old_vertex = vertex;
                } else {
                    if self.vertices.len() < index {
                        self.vertices.reserve(index - self.vertices.len() + 1);
                    }
                    self.vertices[index] = vertex;
                }
            }
            _ => {
                self.vertices.push(vertex);
                self.state = Chaos;
            }
        }
    }

    fn get_vertex(&mut self, index: VertexIndex) -> Option<&Vertex> {
        match self.state {
            GraphState::IndexOrdered => self.vertices.get(index as usize),
            _ => self.vertices.iter().find(|vertex| vertex.index == index),
        }
    }

    fn get_vertex_mut(&mut self, index: VertexIndex) -> Option<&mut Vertex> {
        match self.state {
            GraphState::IndexOrdered => self.vertices.get_mut(index as usize),
            _ => self
                .vertices
                .iter_mut()
                .find(|vertex| vertex.index == index),
        }
    }

    pub fn add_arc(&mut self, start: VertexIndex, end: VertexIndex) {
        if let Some(start_vertex) = self.get_vertex_mut(start) {
            start_vertex.add_edge(end);
        } else {
            let mut vertex = Vertex::new(start, 0);
            vertex.add_edge(end);
            self.add_vertex(vertex);
        }
    }

    pub fn add_edge(&mut self, start: VertexIndex, end: VertexIndex) {
        self.add_arc(start, end);
        self.add_arc(end, start);
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
            vertex.edges_to.sort();
            vertex.edges_to.dedup();
        }
    }

    pub fn set_colours(&mut self, colours: &[Colour]) {
        for (index, colour) in colours.iter().enumerate() {
            if let Some(vertex) = self.get_vertex_mut(index as VertexIndex) {
                vertex.colour = *colour;
            } else {
                self.add_vertex(Vertex::new(index as VertexIndex, *colour));
            }
        }

        self.keep_state_auto = false;
        self.state = GraphState::ColourGroupedOrdered;
    }

    pub fn order(&mut self, order: &[VertexIndex]) {
        let mut ordered_vertices = Vec::with_capacity(self.vertices.len());
        for index in order {
            if let Some(vertex) = self.get_vertex(*index) {
                ordered_vertices.push(vertex.clone());
            } else {
                ordered_vertices.push(Vertex::new(*index, 0));
            }
        }

        self.vertices = ordered_vertices;
        self.keep_state_auto = false;
        self.state = GraphState::IndexOrdered;
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

        if self.keep_state_auto {
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

        let mut last_colour = -1;
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
    fn correct_nauty_repr() {
        let mut graph = Graph::new_empty(8);
        graph.add_edge(0, 1);
        graph.add_edge(0, 3);
        graph.add_edge(0, 4);
        graph.add_edge(1, 2);
        graph.add_edge(1, 5);
        graph.add_edge(2, 3);
        graph.add_edge(2, 6);
        graph.add_edge(3, 7);
        graph.add_edge(4, 5);
        graph.add_edge(4, 7);
        graph.add_edge(5, 6);
        graph.add_edge(6, 7);

        let order = [2, 0, 1, 3, 4, 5, 6, 7];
        let colours = [1, 2, 2, 2, 2, 2, 2, 2];
        graph.order(&order);
        graph.set_colours(&colours);

        let mut nauty_graph = graph.prepare_nauty();
        assert_eq!(nauty_graph.vertex_order, order);
        assert_eq!(nauty_graph.partition, [0, 1, 1, 1, 1, 1, 1, 0]);

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
    }
}
