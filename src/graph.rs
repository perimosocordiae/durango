use crate::data::{ALL_DIRECTIONS, AxialCoord, HexDirection, HexMap, Node};
use std::collections::VecDeque;

#[derive(Default, Clone)]
pub struct HexGraph {
    // Adjacency list, in the same order as map.nodes.
    adj: Vec<[usize; 6]>,
    // Distances to any finish hex.
    pub dists: Vec<i32>,
    // Max valid distance in `dists`.
    pub max_dist: i32,
}

impl HexGraph {
    pub fn new(map: &HexMap) -> Self {
        let adj = create_adjacencies(&map.nodes);
        let dists = create_hex_distances(&map.nodes, &adj, map.finish_idx);
        let max_dist = dists
            .iter()
            .filter(|&&d| d < i32::MAX)
            .max()
            .cloned()
            .unwrap_or(0);
        Self {
            adj,
            dists,
            max_dist,
        }
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
                if nbr_idx < self.adj.len() {
                    Some((nbr_idx, ALL_DIRECTIONS[i]))
                } else {
                    None
                }
            })
    }
}

fn create_adjacencies(nodes: &[(AxialCoord, Node)]) -> Vec<[usize; 6]> {
    nodes
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
        .collect()
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
    use crate::data::LayoutInfo;
    use assert_matches::assert_matches;

    #[test]
    fn neighbors() {
        let map = HexMap::create_custom(&[
            LayoutInfo::new('B', 1, 0, 0),
            LayoutInfo::new('C', 0, 3, -7),
        ])
        .unwrap();
        let idx = map.node_idx(AxialCoord { q: 0, r: 0 }).unwrap();
        assert_eq!(idx, 22);
        let graph = HexGraph::new(&map);
        let nbrs = graph.neighbor_indices(idx).collect::<Vec<_>>();
        assert_eq!(nbrs.len(), 6);
        assert_matches!(nbrs[0], (33, HexDirection::NorthEast));
        assert_matches!(nbrs[1], (34, HexDirection::East));
        assert_matches!(nbrs[2], (23, HexDirection::SouthEast));
        assert_matches!(nbrs[3], (12, HexDirection::SouthWest));
        assert_matches!(nbrs[4], (11, HexDirection::West));
        assert_matches!(nbrs[5], (21, HexDirection::NorthWest));
    }
}
