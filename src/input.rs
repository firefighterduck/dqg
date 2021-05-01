use std::io::{self, Stdin, Write};

use crate::graph::{Graph, VertexIndex};

pub fn read_graph(stdin: &Stdin) -> Result<Graph, io::Error> {
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

pub fn read_vertex(
    index: VertexIndex,
    graph: &mut Graph,
    stdin: &Stdin,
) -> Result<bool, io::Error> {
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
