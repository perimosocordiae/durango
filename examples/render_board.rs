use clap::Parser;
use durango::data;

// Usage:
// cargo run --example render_board -- -l 'A,0,4;A,5,1' | neato -Tsvg | display

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = "A,0,3;A,0,3", value_delimiter = ';')]
    layout: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let layout_csv = format!("board,bottom,next_side\n{}", args.layout.join("\n"));
    let layout = data::load_from_csv::<data::LayoutInfo>(&layout_csv).unwrap();
    let nodes = data::load_nodes(&layout);
    println!("digraph {{");
    // Iterate over all nodes except the first one.
    for (i, node) in nodes.iter().enumerate().skip(1) {
        node.print_dot(i);
    }
    println!("}}");
}
