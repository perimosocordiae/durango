use clap::Parser;
use durango::data;

// Usage:
// cargo run --example render_board -- -l 'A,0,4;A,5,1' | neato -Tsvg | display
// cargo run --example render_board -- -l 'A,0,4;A,5,1' -f svg | display

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value = "B,0,3;B,0,3", value_delimiter = ';')]
    layout: Vec<String>,
    #[clap(short, long, default_value = "dot")]
    format: String,
}

fn dump_dot(nodes: &[data::Node]) {
    println!("digraph {{");
    println!("  overlap=false;");
    println!("  node [style=filled];");
    // Iterate over all nodes except the first one.
    for (i, node) in nodes.iter().enumerate().skip(1) {
        node.print_dot(i);
    }
    println!("}}");
}

fn axial_to_center(q: i32, r: i32, size: f32) -> (f32, f32) {
    let x = size * (3.0_f32).sqrt() * (q as f32 + r as f32 / 2.0);
    let y = size * 1.5 * r as f32;
    (x, y)
}

fn axial_to_polygon(q: i32, r: i32, size: f32) -> String {
    let (cx, cy) = axial_to_center(q, r, size);
    let mut points = Vec::new();
    for i in 0..6 {
        let angle = std::f32::consts::PI / 3.0 * i as f32 + std::f32::consts::PI / 6.0;
        let x = cx + size * angle.cos();
        let y = cy + size * angle.sin();
        points.push(format!("{},{}", x, y));
    }
    points.join(" ")
}

fn axial_neighbor(q: i32, r: i32, direction: usize) -> (i32, i32) {
    let directions = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
    let (dq, dr) = directions[direction];
    (q + dq, r + dr)
}

fn dump_svg(nodes: &[data::Node], size: f32) {
    // DFS traversal from index 1.
    let mut stack = vec![(1, (0, 0))]; // (index, coords)
    let mut visited = vec![false; nodes.len()];
    visited[0] = true; // Skip index 0.
    visited[1] = true;
    let mut min_center = (f32::INFINITY, f32::INFINITY);
    let mut max_center = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut elements = Vec::new();
    while let Some((i, (q, r))) = stack.pop() {
        let node = &nodes[i];
        let (cx, cy) = axial_to_center(q, r, size);
        if cx < min_center.0 {
            min_center.0 = cx;
        }
        if cy < min_center.1 {
            min_center.1 = cy;
        }
        if cx > max_center.0 {
            max_center.0 = cx;
        }
        if cy > max_center.1 {
            max_center.1 = cy;
        }

        elements.push(format!(
            "<g id=\"node{i}\" class=\"hex\">
<polygon points=\"{}\" fill=\"{}\" stroke=\"black\" stroke-width=\"2\" />
<text x=\"{cx}\" y=\"{cy}\" font-size=\"{}\" dominant-baseline=\"middle\" text-anchor=\"middle\">{}</text>
</g>",
            axial_to_polygon(q, r, size),
            node.color(),
            size / 2.0,
            node.cost
        ));
        for (dir, &neighbor) in node.neighbors.iter().enumerate() {
            if !visited[neighbor] {
                stack.push((neighbor, axial_neighbor(q, r, dir)));
                visited[neighbor] = true;
            }
        }
    }

    let margin = size * 1.1;
    let width = (max_center.0 - min_center.0) + 2.0 * margin;
    let height = (max_center.1 - min_center.1) + 2.0 * margin;
    println!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\">",
        min_center.0 - margin,
        min_center.1 - margin,
        width,
        height
    );
    for element in elements.into_iter() {
        println!("{}", element);
    }
    println!("</svg>");
}

fn main() {
    let args = Args::parse();
    let layout_csv = format!("board,bottom,next_side\n{}", args.layout.join("\n"));
    let layout = data::load_from_csv::<data::LayoutInfo>(&layout_csv).unwrap();
    let nodes = data::load_nodes(&layout);
    if args.format == "dot" {
        dump_dot(&nodes);
    } else if args.format == "svg" {
        dump_svg(&nodes, 40.0);
    } else {
        eprintln!("Unsupported format: {}", args.format);
    }
}
