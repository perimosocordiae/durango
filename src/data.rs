use serde::{Deserialize, Serialize};

pub fn load_from_csv<T: for<'de> Deserialize<'de>>(
    data: &str,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(data.as_bytes());
    let mut out = Vec::new();
    for result in rdr.deserialize::<T>() {
        let record: T = result?;
        out.push(record);
    }
    Ok(out)
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum HexDirection {
    NorthEast,
    East,
    SouthEast,
    SouthWest,
    West,
    NorthWest,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Terrain {
    Invalid, // Invalid terrain
    Jungle,  // Jungle movement
    Desert,  // Desert movement
    Water,   // Water movement
    Village, // Trash card(s)
    Swamp,   // Discard card(s)
    Cave,    // Get a bonus
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    terrain: Terrain,
    cost: u8,
    // Indices of neighboring nodes, in HexDirection order.
    neighbors: [usize; 6],
}
impl Node {
    pub fn print_dot(&self, idx: usize) -> () {
        println!("  N{} [label=\"{:?} {}\"]", idx, self.terrain, self.cost);
        for neighbor in self.neighbors.iter() {
            if *neighbor != 0 {
                println!("  N{} -> N{}", idx, neighbor);
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LayoutInfo {
    board: char,
    bottom: u8,
    next_side: u8,
}
impl LayoutInfo {
    pub fn new(board: char, bottom: u8, next_side: u8) -> Self {
        Self {
            board,
            bottom,
            next_side,
        }
    }
}

fn load_board(board: char) -> Vec<Node> {
    match board {
        'A' => load_from_csv::<Node>(include_str!("../boards/A.csv")).unwrap(),
        'B' => load_from_csv::<Node>(include_str!("../boards/B.csv")).unwrap(),
        'C' => load_from_csv::<Node>(include_str!("../boards/C.csv")).unwrap(),
        'D' => load_from_csv::<Node>(include_str!("../boards/D.csv")).unwrap(),
        'E' => load_from_csv::<Node>(include_str!("../boards/E.csv")).unwrap(),
        'F' => load_from_csv::<Node>(include_str!("../boards/F.csv")).unwrap(),
        'G' => load_from_csv::<Node>(include_str!("../boards/G.csv")).unwrap(),
        'H' => load_from_csv::<Node>(include_str!("../boards/H.csv")).unwrap(),
        'I' => load_from_csv::<Node>(include_str!("../boards/I.csv")).unwrap(),
        'J' => load_from_csv::<Node>(include_str!("../boards/J.csv")).unwrap(),
        'K' => load_from_csv::<Node>(include_str!("../boards/K.csv")).unwrap(),
        'L' => load_from_csv::<Node>(include_str!("../boards/L.csv")).unwrap(),
        'M' => load_from_csv::<Node>(include_str!("../boards/M.csv")).unwrap(),
        'N' => load_from_csv::<Node>(include_str!("../boards/N.csv")).unwrap(),
        _ => panic!("Invalid board"),
    }
}

fn side_offsets(side: u8) -> [usize; 4] {
    match side {
        0 => [0, 1, 2, 3],
        1 => [0, 4, 9, 15],
        2 => [15, 22, 28, 33],
        3 => [33, 34, 35, 36],
        4 => [36, 32, 27, 21],
        5 => [21, 14, 8, 3],
        _ => panic!("Invalid side"),
    }
}

pub fn load_nodes(layout: &[LayoutInfo]) -> Vec<Node> {
    let mut result = vec![Node {
        terrain: Terrain::Invalid,
        cost: 0,
        neighbors: [0; 6],
    }];
    let mut prev_start = 1;
    for (i, info) in layout.iter().enumerate() {
        let mut board_nodes = load_board(info.board);
        // Update node indices.
        let to_add = result.len() - 1;
        for node in &mut board_nodes {
            for neighbor in &mut node.neighbors {
                if *neighbor != 0 {
                    *neighbor += to_add;
                }
            }
        }
        if i > 0 {
            // Connect the boards.
            let prev_side = layout[i - 1].next_side;
            let curr_side = info.bottom;
            let prev_offsets = side_offsets(prev_side);
            let curr_offsets = side_offsets(curr_side);
            let curr_start = result.len();
            let prev0 = prev_start + prev_offsets[0];
            let prev1 = prev_start + prev_offsets[1];
            let prev2 = prev_start + prev_offsets[2];
            let prev3 = prev_start + prev_offsets[3];
            // TODO: Set the correct neighbors based on info.bottom
            // The current setup works for info.bottom == 0 only.
            board_nodes[curr_offsets[0]].neighbors[2] = prev0;
            board_nodes[curr_offsets[1]].neighbors[2] = prev1;
            board_nodes[curr_offsets[1]].neighbors[3] = prev0;
            board_nodes[curr_offsets[2]].neighbors[2] = prev2;
            board_nodes[curr_offsets[2]].neighbors[3] = prev1;
            board_nodes[curr_offsets[3]].neighbors[2] = prev3;
            board_nodes[curr_offsets[3]].neighbors[3] = prev2;
            // TODO: fix this based on next_side. Current setup assumes ns=3.
            result[prev0].neighbors[5] = curr_start + curr_offsets[0];
            result[prev0].neighbors[0] = curr_start + curr_offsets[1];
            result[prev1].neighbors[5] = curr_start + curr_offsets[1];
            result[prev1].neighbors[0] = curr_start + curr_offsets[2];
            result[prev2].neighbors[5] = curr_start + curr_offsets[2];
            result[prev2].neighbors[0] = curr_start + curr_offsets[3];
            result[prev3].neighbors[5] = curr_start + curr_offsets[3];

            prev_start = curr_start;
        }
        result.append(&mut board_nodes);
    }
    result
}

fn easy_1() -> [LayoutInfo; 6] {
    [
        LayoutInfo::new('B', 5, 2),
        LayoutInfo::new('C', 0, 3),
        LayoutInfo::new('G', 2, 1),
        LayoutInfo::new('K', 1, 4),
        LayoutInfo::new('J', 1, 3),
        LayoutInfo::new('N', 0, 3),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_board() {
        let nodes = load_board('A');
        assert_eq!(nodes.len(), 37);
    }

    #[test]
    fn whole_layout() {
        let nodes = load_nodes(&[LayoutInfo::new('A', 0, 3), LayoutInfo::new('A', 0, 3)]);
        assert_eq!(nodes.len(), 1 + 37 + 37);
    }
}
