use std::collections::HashMap;

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

#[derive(
    Serialize, Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub struct AxialCoord {
    pub q: i32,
    pub r: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum HexDirection {
    NorthEast,
    East,
    SouthEast,
    SouthWest,
    West,
    NorthWest,
}
impl HexDirection {
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::NorthEast,
            1 => Self::East,
            2 => Self::SouthEast,
            3 => Self::SouthWest,
            4 => Self::West,
            5 => Self::NorthWest,
            _ => panic!("Invalid index"),
        }
    }
    pub fn reverse(&self) -> Self {
        match self {
            Self::NorthEast => Self::SouthWest,
            Self::East => Self::West,
            Self::SouthEast => Self::NorthWest,
            Self::SouthWest => Self::NorthEast,
            Self::West => Self::East,
            Self::NorthWest => Self::SouthEast,
        }
    }
    pub fn neighbor_coord(&self, coord: AxialCoord) -> AxialCoord {
        let (dq, dr) = match self {
            Self::East => (1, 0),
            Self::West => (-1, 0),
            Self::NorthEast => (1, -1),
            Self::NorthWest => (0, -1),
            Self::SouthEast => (0, 1),
            Self::SouthWest => (-1, 1),
        };
        AxialCoord {
            q: coord.q + dq,
            r: coord.r + dr,
        }
    }
    pub fn all_directions() -> [Self; 6] {
        [
            Self::NorthEast,
            Self::East,
            Self::SouthEast,
            Self::SouthWest,
            Self::West,
            Self::NorthWest,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
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
    pub terrain: Terrain,
    pub cost: u8,
}
impl Node {
    pub fn color(&self) -> &'static str {
        match self.terrain {
            Terrain::Jungle => "green",
            Terrain::Desert => "yellow",
            Terrain::Water => "blue",
            Terrain::Village => "red",
            Terrain::Swamp => "gray",
            _ => "white",
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SavedNode {
    #[serde(flatten)]
    node: Node,
    #[serde(flatten)]
    coord: AxialCoord,
}

#[derive(Serialize, Deserialize)]
pub struct LayoutInfo {
    board: char,
    rotation: u8, // 0-5, clockwise from bottom
    #[serde(flatten)]
    center: AxialCoord,
}
impl LayoutInfo {
    pub fn new(board: char, rotation: u8, q: i32, r: i32) -> Self {
        Self {
            board,
            rotation,
            center: AxialCoord { q, r },
        }
    }
}

fn load_board(
    board: char,
) -> Result<Vec<SavedNode>, Box<dyn std::error::Error>> {
    match board {
        'A' => load_from_csv::<SavedNode>(include_str!("../boards/A.csv")),
        'B' => load_from_csv::<SavedNode>(include_str!("../boards/B.csv")),
        'C' => load_from_csv::<SavedNode>(include_str!("../boards/C.csv")),
        'D' => load_from_csv::<SavedNode>(include_str!("../boards/D.csv")),
        'E' => load_from_csv::<SavedNode>(include_str!("../boards/E.csv")),
        'F' => load_from_csv::<SavedNode>(include_str!("../boards/F.csv")),
        'G' => load_from_csv::<SavedNode>(include_str!("../boards/G.csv")),
        'H' => load_from_csv::<SavedNode>(include_str!("../boards/H.csv")),
        'I' => load_from_csv::<SavedNode>(include_str!("../boards/I.csv")),
        'J' => load_from_csv::<SavedNode>(include_str!("../boards/J.csv")),
        'K' => load_from_csv::<SavedNode>(include_str!("../boards/K.csv")),
        'L' => load_from_csv::<SavedNode>(include_str!("../boards/L.csv")),
        'M' => load_from_csv::<SavedNode>(include_str!("../boards/M.csv")),
        'N' => load_from_csv::<SavedNode>(include_str!("../boards/N.csv")),
        _ => Err("Invalid board".into()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct HexMap {
    pub nodes: HashMap<AxialCoord, Node>,
}

pub fn load_nodes(layout: &[LayoutInfo]) -> HexMap {
    let mut nodes = HashMap::new();
    for info in layout {
        let board_nodes = load_board(info.board).unwrap();
        for mut tmp in board_nodes.into_iter() {
            let coord = &mut tmp.coord;
            // Rotate coord based on info.rotation
            for _ in 0..info.rotation {
                let q = coord.q;
                let r = coord.r;
                coord.q = -r;
                coord.r = q + r;
            }
            // Translate coord based on info.center
            coord.q += info.center.q;
            coord.r += info.center.r;
            nodes.insert(*coord, tmp.node);
        }
    }
    HexMap { nodes }
}

pub fn easy_1() -> [LayoutInfo; 6] {
    [
        LayoutInfo::new('B', 0, 0, 0),
        LayoutInfo::new('C', 0, 0, 3),
        LayoutInfo::new('G', 0, 2, 1),
        LayoutInfo::new('K', 0, 1, 4),
        LayoutInfo::new('J', 0, 1, 3),
        LayoutInfo::new('N', 0, 0, 3),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_board() {
        let nodes = load_board('A').unwrap();
        assert_eq!(nodes.len(), 36);
    }

    #[test]
    fn whole_layout() {
        let map = load_nodes(&[
            LayoutInfo::new('A', 0, 0, 0),
            LayoutInfo::new('A', 0, 9, 9),
        ]);
        assert_eq!(map.nodes.len(), 36 + 36);
    }
}
