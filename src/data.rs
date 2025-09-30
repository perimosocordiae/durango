use std::collections::VecDeque;

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Node {
    pub terrain: Terrain,
    pub cost: u8,
    pub board_idx: u8,
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
        "easy1" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy1.csv"))
        }
        "easy2" => {
            load_from_csv::<LayoutInfo>(include_str!("../layouts/easy2.csv"))
        }
        _ => Err("Unknown layout".into()),
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HexMap {
    // Mapping from axial coordinates to nodes, in sorted order.
    pub nodes: Vec<(AxialCoord, Node)>,
    // Index of the "finish" board.
    finish_idx: u8,
    // Adjacency list, in the same order as nodes.
    #[serde(skip)]
    adj: Vec<[usize; 6]>,
    // Distances to any finish hex.
    #[serde(skip)]
    pub dists: Vec<i32>,
    #[serde(skip)]
    pub max_dist: i32,
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
        let adj = create_adjacencies(&nodes);
        let dists = create_hex_distances(&nodes, &adj, finish_idx);
        let max_dist = dists
            .iter()
            .filter(|&&d| d < i32::MAX)
            .max()
            .cloned()
            .unwrap_or(0);
        Ok(HexMap {
            nodes,
            finish_idx,
            adj,
            dists,
            max_dist,
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
    /// Get the neighboring nodes of a given coordinate.
    pub fn neighbors_of(
        &self,
        coord: AxialCoord,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &Node)> {
        self.neighbors_of_idx(
            self.nodes
                .binary_search_by_key(&coord, |(c, _)| *c)
                .unwrap_or(usize::MAX),
        )
    }
    /// Get the neighboring nodes of a given node index.
    pub fn neighbors_of_idx(
        &self,
        idx: usize,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &Node)> {
        self.neighbor_indices(idx).map(|(nbr_idx, dir)| {
            let (pos, node) = &self.nodes[nbr_idx];
            (dir, *pos, node)
        })
    }
    /// Get the neighboring node indices of a given node index.
    pub fn neighbor_indices(
        &self,
        idx: usize,
    ) -> impl Iterator<Item = (usize, HexDirection)> + '_ {
        self.adj
            .get(idx)
            .unwrap_or(&[usize::MAX; 6])
            .iter()
            .enumerate()
            .filter_map(|(i, &nbr_idx)| {
                if nbr_idx < self.nodes.len() {
                    Some((nbr_idx, ALL_DIRECTIONS[i]))
                } else {
                    None
                }
            })
    }
    /// Get the node index of a given coordinate.
    pub fn node_idx(&self, coord: AxialCoord) -> Option<usize> {
        self.nodes.binary_search_by_key(&coord, |(c, _)| *c).ok()
    }
    /// Get the node at a given coordinate.
    pub fn node_at(&self, coord: AxialCoord) -> Option<&Node> {
        self.node_idx(coord).map(|idx| &self.nodes[idx].1)
    }
    /// Get the node at a given index.
    pub fn node_at_idx(&self, idx: usize) -> Option<&Node> {
        self.nodes.get(idx).map(|(_, node)| node)
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
    pub fn all_nodes(&self) -> impl Iterator<Item = &(AxialCoord, Node)> {
        self.nodes.iter()
    }
}

fn create_adjacencies(nodes: &[(AxialCoord, Node)]) -> Vec<[usize; 6]> {
    return nodes
        .iter()
        .map(|(pos, _)| {
            let mut neighbors = [0; 6];
            for (i, dir) in ALL_DIRECTIONS.iter().enumerate() {
                let nbr_pos = dir.neighbor_coord(*pos);
                neighbors[i] = nodes
                    .binary_search_by_key(&nbr_pos, |(c, _)| *c)
                    .unwrap_or(usize::MAX);
            }
            neighbors
        })
        .collect();
}

/// Returns a distance (in terms of # hexes, not move cost) for every node.
fn create_hex_distances(
    nodes: &[(AxialCoord, Node)],
    adj: &[[usize; 6]],
    finish_idx: u8,
) -> Vec<i32> {
    // Run BFS from the finish nodes.
    let mut queue = nodes
        .iter()
        .enumerate()
        .filter_map(|(i, (_, node))| {
            if node.board_idx == finish_idx {
                Some((i, 0))
            } else {
                None
            }
        })
        .collect::<VecDeque<(usize, i32)>>();
    let mut dists = vec![i32::MAX; nodes.len()];
    for &(i, _) in &queue {
        dists[i] = 0;
    }
    while let Some((idx, dist)) = queue.pop_front() {
        let next_dist = dist + 1;
        for &nbr_idx in &adj[idx] {
            if let Some(d) = dists.get(nbr_idx)
                && *d > next_dist
                && nodes[nbr_idx].1.cost < 10
            {
                dists[nbr_idx] = next_dist;
                queue.push_back((nbr_idx, next_dist));
            }
        }
    }
    dists
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

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

    #[test]
    fn neighbors() {
        let map = HexMap::create_custom(&[
            LayoutInfo::new('B', 1, 0, 0),
            LayoutInfo::new('C', 0, 3, -7),
        ])
        .unwrap();
        let nbrs = map
            .neighbors_of(AxialCoord { q: 0, r: 0 })
            .collect::<Vec<_>>();
        assert_eq!(nbrs.len(), 6);
        assert_matches!(
            nbrs[0],
            (
                HexDirection::NorthEast,
                AxialCoord { q: 1, r: -1 },
                Node {
                    terrain: Terrain::Jungle,
                    cost: 1,
                    board_idx: 0
                }
            )
        );
        assert_matches!(
            nbrs[1],
            (HexDirection::East, AxialCoord { q: 1, r: 0 }, _)
        );
        assert_matches!(
            nbrs[2],
            (HexDirection::SouthEast, AxialCoord { q: 0, r: 1 }, _)
        );
        assert_matches!(
            nbrs[3],
            (HexDirection::SouthWest, AxialCoord { q: -1, r: 1 }, _)
        );
        assert_matches!(
            nbrs[4],
            (HexDirection::West, AxialCoord { q: -1, r: 0 }, _)
        );
        assert_matches!(
            nbrs[5],
            (HexDirection::NorthWest, AxialCoord { q: 0, r: -1 }, _)
        );
    }
}
