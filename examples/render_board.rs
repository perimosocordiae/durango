use clap::Parser;
use durango::data::{self, AxialCoord};
use durango::data::{HexDirection, HexMap, LayoutInfo, Terrain};

// Usage:
// cargo run --example render_board -- -f dot | neato -Tsvg | display
// cargo run --example render_board -- -f svg | display

#[derive(Parser)]
struct Args {
    #[clap(
        short,
        long,
        default_value = "B,0,0,0;B,3,7,0",
        value_delimiter = ';'
    )]
    layout: Vec<String>,
    #[clap(short, long)]
    preset: Option<String>,
    #[clap(short, long, default_value = "dot")]
    format: String,
}

fn coord_to_string(coord: &AxialCoord) -> String {
    format!("q{}r{}", coord.q + 1000, coord.r + 1000)
}

fn dump_dot(map: &HexMap) {
    println!("digraph {{");
    println!("  overlap=false;");
    println!("  node [style=filled];");
    for (coord, node) in &map.nodes {
        if matches!(node.terrain, Terrain::Invalid) {
            continue;
        }
        println!(
            "  {} [label=\"{},{}: {}\",fillcolor={}]",
            coord_to_string(coord),
            coord.q,
            coord.r,
            node.cost,
            node.color()
        );
        for dir in HexDirection::all_directions() {
            let next_pos = dir.neighbor_coord(*coord);
            if let Some(neighbor) = map.nodes.get(&next_pos) {
                if !matches!(neighbor.terrain, Terrain::Invalid) {
                    println!(
                        "  {} -> {}",
                        coord_to_string(coord),
                        coord_to_string(&next_pos)
                    );
                }
            }
        }
    }
    println!("}}");
}

fn axial_to_center(pos: &AxialCoord, size: f32) -> (f32, f32) {
    let x = size * (3.0_f32).sqrt() * (pos.q as f32 + pos.r as f32 / 2.0);
    let y = size * 1.5 * pos.r as f32;
    (x, y)
}

fn axial_to_polygon(pos: &AxialCoord, size: f32) -> String {
    let (cx, cy) = axial_to_center(pos, size);
    let mut points = Vec::new();
    for i in 0..6 {
        let angle =
            std::f32::consts::PI / 3.0 * i as f32 + std::f32::consts::PI / 6.0;
        let x = cx + size * angle.cos();
        let y = cy + size * angle.sin();
        points.push(format!("{},{}", x, y));
    }
    points.join(" ")
}

fn dump_svg(map: &HexMap, size: f32) {
    let mut min_center = (f32::INFINITY, f32::INFINITY);
    let mut max_center = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut elements = Vec::new();
    for (i, (coord, node)) in map.nodes.iter().enumerate() {
        let (cx, cy) = axial_to_center(coord, size);
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
            axial_to_polygon(coord, size),
            node.color(),
            size / 2.0,
            node.cost
        ));
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

fn render(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let map = match &args.preset {
        Some(name) => HexMap::create_named(&name),
        None => {
            let layout_csv =
                format!("board,rotation,q,r\n{}", args.layout.join("\n"));
            let layout = data::load_from_csv::<LayoutInfo>(&layout_csv)?;
            HexMap::create_custom(&layout)
        }
    }?;
    if args.format == "dot" {
        dump_dot(&map);
    } else if args.format == "svg" {
        dump_svg(&map, 30.0);
    } else {
        eprintln!("Unsupported format: {}", args.format);
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    match render(&args) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}
