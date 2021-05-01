mod graph;
use std::{
    error::Error,
    io::{self, Stdin, Write},
    os::raw::c_int,
    slice::from_raw_parts,
    usize,
};

use graph::*;
use libffi::high::ClosureMut6;
use nauty_Traces_sys::{densenauty, optionblk, statsblk};

fn read_graph(stdin: &Stdin) -> Result<Graph, io::Error> {
    let mut buffer = String::new();
    let number_of_vertices;

    println!("Input graph size:");
    loop {
        print!("n=");
        io::stdout().flush()?;
        stdin.read_line(&mut buffer)?;

        match buffer.trim().parse::<usize>() {
            Ok(n) => {
                number_of_vertices = n;
                break;
            }
            Err(_) => println!("Please insert only natural numbers in decimal!"),
        }

        buffer.clear();
    }

    Ok(Graph::new_ordered(number_of_vertices))
}

fn read_vertex(index: VertexIndex, graph: &mut Graph, stdin: &Stdin) -> Result<bool, io::Error> {
    let mut line_buffer = String::new();
    let mut should_continue = true;

    'input: loop {
        print!("Edges from {}: ", index);
        io::stdout().flush()?;

        line_buffer.clear();
        stdin.read_line(&mut line_buffer)?;
        for input_part in line_buffer.split_whitespace() {
            if let Ok(end) = input_part.parse::<VertexIndex>() {
                if end < graph.size() as i32 {
                    graph.add_edge(index, end);
                } else {
                    println!(
                        "Please only input valid vertex indices (i.e. between 0 and {})!",
                        graph.size()
                    );
                    continue 'input;
                }
            } else if input_part.chars().next().unwrap_or(' ') == ';' {
                break 'input;
            } else if input_part.chars().next().unwrap_or(' ') == '.' {
                should_continue = false;
                break 'input;
            } else {
                break;
            }
        }

        println!("Please insert the edges from vertex {} in this format: `Edges from {}: #1 #2 #3 (;|.)` where \
#i is the vertex number of the end node of an edge from this vertex. Also end the line with a `;` to continue \
with the next vertex or a `.` to end inputting edges.", index, index);
    }

    Ok(should_continue)
}

fn call_nauty(mut nauty_graph: NautyGraph) -> Vec<Vec<c_int>> {
    let (n, m) = nauty_graph.graph_repr_sizes();
    let mut stats = statsblk::default();
    let mut orbits = vec![0 as c_int; n];
    let mut options = optionblk::default();
    let mut generators = Vec::new();
    // Limit how long the closure can reference generators so that we can return it afterwards.
    {
        let mut userautomproc =
            |_count, generator_ptr: *mut c_int, _orbits, _numorbits, _stabvertex, n: c_int| {
                let mut generator = Vec::with_capacity(n as usize);
                let generator_raw = unsafe { from_raw_parts(generator_ptr, n as usize) };

                for vertex in generator_raw {
                    generator.push(*vertex);
                }

                generators.push(generator);
            };
        let userautomproc = ClosureMut6::new(&mut userautomproc);
        options.userautomproc = Some(*userautomproc.code_ptr());

        unsafe {
            densenauty(
                nauty_graph.adjacency_matrix.as_mut_ptr(),
                nauty_graph.vertex_order.as_mut_ptr(),
                nauty_graph.partition.as_mut_ptr(),
                orbits.as_mut_ptr(),
                &mut options,
                &mut stats,
                m as c_int,
                n as c_int,
                std::ptr::null_mut(),
            );
        }
    }
    generators
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut graph = read_graph(&stdin)?;

    for i in 0..graph.size() {
        if !read_vertex(i as VertexIndex, &mut graph, &stdin)? {
            break;
        }
    }

    let nauty_graph = graph.prepare_nauty();
    assert!(nauty_graph.check_valid());
    let generators = call_nauty(nauty_graph);

    for generator in generators {
        println!("{:?}", generator);
    }

    Ok(())
}
