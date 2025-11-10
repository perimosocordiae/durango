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
    Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct AxialCoord {
    pub q: i32,
    pub r: i32,
}
impl AxialCoord {
    pub fn is_adjacent(&self, other: AxialCoord) -> bool {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = (self.q + self.r - other.q - other.r).abs();
        // max(dq, dr, ds) == 1
        (dq <= 1) && (dr <= 1) && (ds <= 1) && (dq + dr + ds == 2)
    }
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

pub(crate) static ALL_DIRECTIONS: [HexDirection; 6] = [
    HexDirection::NorthEast,
    HexDirection::East,
    HexDirection::SouthEast,
    HexDirection::SouthWest,
    HexDirection::West,
    HexDirection::NorthWest,
];

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum Terrain {
    Invalid, // Invalid terrain
    Jungle,  // Jungle movement
    Desert,  // Desert movement
    Water,   // Water movement
    Village, // Trash card(s)
    Swamp,   // Discard card(s)
    Cave,    // Get a bonus
}
impl Terrain {
    fn as_u8(&self) -> u8 {
        match self {
            Terrain::Invalid => 0,
            Terrain::Jungle => 1,
            Terrain::Desert => 2,
            Terrain::Water => 3,
            Terrain::Village => 4,
            Terrain::Swamp => 5,
            Terrain::Cave => 6,
        }
    }
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Terrain::Invalid),
            1 => Some(Terrain::Jungle),
            2 => Some(Terrain::Desert),
            3 => Some(Terrain::Water),
            4 => Some(Terrain::Village),
            5 => Some(Terrain::Swamp),
            6 => Some(Terrain::Cave),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum BonusToken {
    Jungle(u8),
    Desert(u8),
    Water(u8),
    DrawCard,
    TrashCard,
    ReplaceHand,
    DoubleUse, // TODO: teach agents to use
    ShareHex,  // TODO: teach agents to use
    FreeMove,
    SwapSymbol, // TODO: teach agents to use
}
impl BonusToken {
    pub fn gold_value(&self) -> u8 {
        match self {
            BonusToken::Desert(v) => *v * 2,
            _ => 0,
        }
    }
}

pub(crate) static ALL_BONUS_TOKENS: [BonusToken; 36] = [
    BonusToken::Jungle(1),
    BonusToken::Jungle(1),
    BonusToken::Jungle(2),
    BonusToken::Jungle(2),
    BonusToken::Jungle(2),
    BonusToken::Jungle(3),
    BonusToken::Jungle(3),
    BonusToken::Desert(1),
    BonusToken::Desert(1),
    BonusToken::Desert(2),
    BonusToken::Desert(2),
    BonusToken::Desert(2),
    BonusToken::Water(1),
    BonusToken::Water(1),
    BonusToken::Water(2),
    BonusToken::Water(2),
    BonusToken::Water(2),
    BonusToken::DrawCard,
    BonusToken::DrawCard,
    BonusToken::DrawCard,
    BonusToken::DrawCard,
    BonusToken::TrashCard,
    BonusToken::TrashCard,
    BonusToken::TrashCard,
    BonusToken::TrashCard,
    BonusToken::ReplaceHand,
    BonusToken::ReplaceHand,
    BonusToken::ReplaceHand,
    BonusToken::DoubleUse,
    BonusToken::DoubleUse,
    BonusToken::ShareHex,
    BonusToken::ShareHex,
    BonusToken::FreeMove,
    BonusToken::FreeMove,
    BonusToken::SwapSymbol,
    BonusToken::SwapSymbol,
];

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub terrain: Terrain,
    pub cost: u8,
    pub board_idx: u8,
}
impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let terrain = self.terrain.as_u8() as u16;
        let cost = self.cost as u16;
        let board_idx = self.board_idx as u16;
        // Layout: TTTTCCCCBBBBBBBB
        let x: u16 = (terrain << 12) | (cost << 8) | (board_idx);
        serializer.serialize_u16(x)
    }
}
impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = u16::deserialize(deserializer)?;
        let terrain_u8 = ((raw >> 12) & 0x0F) as u8;
        let cost = ((raw >> 8) & 0x0F) as u8;
        let board_idx = (raw & 0xFF) as u8;
        let terrain = Terrain::from_u8(terrain_u8).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Invalid terrain value: {terrain_u8}"
            ))
        })?;
        Ok(Node {
            terrain,
            cost,
            board_idx,
        })
    }
}

/// A barrier between two boards.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Barrier {
    pub from_board: usize,
    pub to_board: usize,
    pub terrain: Terrain,
    pub cost: u8,
    // Edges where this barrier exists. This is technically redundant info, as
    // it can be derived from from_board and to_board, but it's convenient to
    // store it to avoid recomputation.
    pub edges: Vec<(AxialCoord, HexDirection)>,
}

pub(crate) static ALL_BARRIER_TYPES: [(Terrain, u8); 6] = [
    (Terrain::Jungle, 1),
    (Terrain::Jungle, 2),
    (Terrain::Desert, 1),
    (Terrain::Water, 1),
    (Terrain::Swamp, 1),
    (Terrain::Swamp, 2),
];

#[derive(Serialize, Deserialize)]
struct SavedNode {
    terrain: Terrain,
    cost: u8,
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
        "first" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/first.csv"))
        }
        "easy1" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy1.csv"))
        }
        "easy2" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy2.csv"))
        }
        "medium1" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/medium1.csv"))
        }
        "medium2" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/medium2.csv"))
        }
        "hard1" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/hard1.csv"))
        }
        "hard2" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/hard2.csv"))
        }
        _ => Err("Unknown layout".into()),
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HexMap {
    // nodes[i] is at coordinate (q[i], r[i]), in sorted order by coordinate.
    qs: Vec<i32>,
    rs: Vec<i32>,
    nodes: Vec<Node>,
    // Index of the "finish" board.
    pub(crate) finish_idx: u8,
}

impl HexMap {
    /// Create a custom map from a layout description.
    pub fn create_custom(
        layout: &[LayoutInfo],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if layout.is_empty() {
            return Err("Cannot create map with an empty layout".into());
        }
        let mut nodes = Vec::new();
        for (i, info) in layout.iter().enumerate() {
            let board_nodes = load_board(info.board)?;
            let board_idx = i as u8;
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
                nodes.push((
                    *coord,
                    Node {
                        terrain: tmp.terrain,
                        cost: tmp.cost,
                        board_idx,
                    },
                ));
            }
        }
        nodes.sort_unstable_by_key(|(coord, _)| *coord);
        // Check if any two nodes overlap.
        for w in nodes.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(format!("Overlapping nodes at {:?}", w[0].0).into());
            }
        }
        let finish_idx = (layout.len() - 1) as u8;
        Ok(HexMap {
            qs: nodes.iter().map(|(coord, _)| coord.q).collect(),
            rs: nodes.iter().map(|(coord, _)| coord.r).collect(),
            nodes: nodes.into_iter().map(|(_, node)| node).collect(),
            finish_idx,
        })
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
        self.node_at(coord).map(|n| n.board_idx) == Some(self.finish_idx)
    }
    /// Get the node index of a given coordinate.
    pub fn node_idx(&self, coord: AxialCoord) -> Option<usize> {
        let start_idx = self.qs.partition_point(|&q| q < coord.q);
        let end_idx =
            self.qs[start_idx..].partition_point(|&q| q == coord.q) + start_idx;
        // TODO: maybe just linear search here.
        self.rs[start_idx..end_idx]
            .binary_search(&coord.r)
            .ok()
            .map(|i| i + start_idx)
    }
    /// Get the node at a given coordinate.
    pub fn node_at(&self, coord: AxialCoord) -> Option<&Node> {
        self.node_idx(coord).map(|idx| &self.nodes[idx])
    }
    /// Get the node at a given index.
    pub fn node_at_idx(&self, idx: usize) -> Option<&Node> {
        self.nodes.get(idx)
    }
    /// Get the coordinate at a given index.
    pub fn coord_at_idx(&self, idx: usize) -> Option<AxialCoord> {
        self.qs
            .get(idx)
            .and_then(|&q| self.rs.get(idx).map(|&r| AxialCoord { q, r }))
    }
    /// Checks if the given coordinate has a node of the given terrain.
    pub fn with_terrain(
        &self,
        coord: AxialCoord,
        terrain: Terrain,
    ) -> Option<&Node> {
        self.node_at(coord).filter(|n| n.terrain == terrain)
    }
    /// Returns an iterator over all nodes in the map.
    pub fn all_nodes(&self) -> impl Iterator<Item = (AxialCoord, &Node)> {
        self.qs
            .iter()
            .zip(self.rs.iter())
            .zip(self.nodes.iter())
            .map(|((&q, &r), node)| (AxialCoord { q, r }, node))
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

    #[test]
    fn named_layout() {
        let map = HexMap::create_named("easy1").unwrap();
        let str = serde_json::to_string(&map).unwrap();
        let map2: HexMap = serde_json::from_str(&str).unwrap();
        assert_eq!(map.nodes.len(), map2.nodes.len());
        assert_eq!(map.finish_idx, map2.finish_idx);
    }
}
