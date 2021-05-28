//! Representation of graphs as well as
//! functionalities to build them from
//! simple building blocks or switch to
//! to a representation understand by nauty.
use custom_debug_derive::Debug;
use std::os::raw::c_int;

mod internal_graph;
pub use internal_graph::{Graph, GraphState, Vertex};

mod nauty_traces_graph;
pub use nauty_traces_graph::{NautyGraph, SparseNautyGraph, TracesGraph};

pub type Colour = c_int;
pub type VertexIndex = c_int;

pub const DEFAULT_COLOR: Colour = c_int::MAX;

#[derive(Debug, PartialEq, Eq)]
pub struct GraphError(VertexIndex);
