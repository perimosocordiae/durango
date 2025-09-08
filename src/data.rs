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

#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AxialCoord {
    pub q: i32,
    pub r: i32,
}
impl std::fmt::Debug for AxialCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.q, self.r)
    }
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
}

static ALL_DIRECTIONS: [HexDirection; 6] = [
    HexDirection::NorthEast,
    HexDirection::East,
    HexDirection::SouthEast,
    HexDirection::SouthWest,
    HexDirection::West,
    HexDirection::NorthWest,
];

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
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
            Terrain::Cave => "brown",
            Terrain::Invalid => "black",
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
        'O' => load_from_csv::<SavedNode>(include_str!("../boards/O.csv")),
        'P' => load_from_csv::<SavedNode>(include_str!("../boards/P.csv")),
        'Q' => load_from_csv::<SavedNode>(include_str!("../boards/Q.csv")),
        'R' => load_from_csv::<SavedNode>(include_str!("../boards/R.csv")),
        'Y' => load_from_csv::<SavedNode>(include_str!("../boards/Y.csv")),
        'Z' => load_from_csv::<SavedNode>(include_str!("../boards/Z.csv")),
        _ => Err(format!("Invalid board: {}", board).into()),
    }
}

fn load_layout(
    name: &str,
) -> Result<Vec<LayoutInfo>, Box<dyn std::error::Error>> {
    match name {
        "easy1" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy1.csv"))
        }
        "easy2" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy2.csv"))
        }
        _ => Err("Unknown layout".into()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct HexMap {
    nodes: HashMap<AxialCoord, Node>,
    finish: Vec<AxialCoord>,
}
impl HexMap {
    /// Create a custom map from a layout description.
    pub fn create_custom(
        layout: &[LayoutInfo],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if layout.is_empty() {
            return Err("Cannot create map with an empty layout".into());
        }
        let last_board = layout.len() - 1;
        let mut nodes = HashMap::new();
        let mut finish = Vec::new();
        for (i, info) in layout.iter().enumerate() {
            let board_nodes = load_board(info.board)?;
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
                // Insert into map, unless there's already something there
                match nodes.entry(*coord) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(tmp.node);
                    }
                    std::collections::hash_map::Entry::Occupied(_) => {
                        return Err("Overlapping boards".into());
                    }
                }
                if i == last_board {
                    finish.push(*coord);
                }
            }
        }
        Ok(HexMap { nodes, finish })
    }
    /// Create a map from a named layout.
    pub fn create_named(
        name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let layout = load_layout(name)?;
        Self::create_custom(&layout)
    }
    /// Check if the given coordinate is a finish node.
    pub fn is_finish(&self, coord: AxialCoord) -> bool {
        self.finish.contains(&coord)
    }
    /// Get the neighboring nodes of a given coordinate.
    pub fn neighbors_of(
        &self,
        coord: AxialCoord,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &Node)> {
        ALL_DIRECTIONS.iter().filter_map(move |dir| {
            let neighbor_pos = dir.neighbor_coord(coord);
            self.nodes
                .get(&neighbor_pos)
                .map(|node| (*dir, neighbor_pos, node))
        })
    }
    pub fn node_at(&self, coord: AxialCoord) -> Option<&Node> {
        self.nodes.get(&coord)
    }
    /// Checks if the given coordinate has a node of the given terrain.
    pub fn with_terrain(
        &self,
        coord: AxialCoord,
        terrain: Terrain,
    ) -> Option<&Node> {
        self.nodes.get(&coord).filter(|n| n.terrain == terrain)
    }
    /// Returns an iterator over all nodes in the map.
    pub fn all_nodes(&self) -> impl Iterator<Item = (&AxialCoord, &Node)> {
        self.nodes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_board() {
        let nodes = load_board('A').unwrap();
        assert_eq!(nodes.len(), 37);
    }

    #[test]
    fn whole_layout() {
        let map = HexMap::create_custom(&[
            LayoutInfo::new('B', 1, 0, 0),
            LayoutInfo::new('C', 0, 3, -7),
        ])
        .unwrap();
        assert_eq!(map.nodes.len(), 74);
    }
}
