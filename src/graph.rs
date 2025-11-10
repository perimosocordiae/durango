use crate::data::{ALL_DIRECTIONS, AxialCoord, HexDirection, HexMap, Node};
use std::collections::{BinaryHeap, VecDeque};

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
        let adj = create_adjacencies(map);
        let dists = create_hex_distances(map, &adj, map.finish_idx);
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
    /// Get the neighboring nodes of a given node index.
    pub fn neighbors_of_idx<'a>(
        &self,
        map: &'a HexMap,
        idx: usize,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &'a Node)> {
        self.neighbor_indices(idx).map(|(nbr_idx, dir)| {
            (
                dir,
                map.coord_at_idx(nbr_idx).unwrap(),
                map.node_at_idx(nbr_idx).unwrap(),
            )
        })
    }
    /// Get the neighboring nodes of a given coordinate.
    pub fn neighbors_of<'a>(
        &self,
        map: &'a HexMap,
        coord: AxialCoord,
    ) -> impl Iterator<Item = (HexDirection, AxialCoord, &'a Node)> {
        let idx = map.node_idx(coord).unwrap_or(usize::MAX);
        self.neighbors_of_idx(map, idx)
    }
    /// Get customized distances to the finish.
    pub fn distances_to_finish(
        &self,
        map: &HexMap,
        cost_fn: impl Fn(&Node) -> f64,
    ) -> Vec<f64> {
        custom_distances(map, &self.adj, map.finish_idx, cost_fn)
    }
}

fn create_adjacencies(map: &HexMap) -> Vec<[usize; 6]> {
    map.all_nodes()
        .map(|(pos, _)| {
            let mut neighbors = [0; 6];
            for (i, dir) in ALL_DIRECTIONS.iter().enumerate() {
                let nbr_pos = dir.neighbor_coord(*pos);
                neighbors[i] = map.node_idx(nbr_pos).unwrap_or(usize::MAX);
            }
            neighbors
        })
        .collect()
}

/// Returns a distance (in terms of # hexes, not move cost) for every node.
fn create_hex_distances(
    map: &HexMap,
    adj: &[[usize; 6]],
    finish_board_idx: u8,
) -> Vec<i32> {
    // Run BFS from the finish nodes.
    let mut queue = map
        .all_nodes()
        .enumerate()
        .filter_map(|(i, (_, node))| {
            if node.board_idx == finish_board_idx {
                Some((i, 0))
            } else {
                None
            }
        })
        .collect::<VecDeque<(usize, i32)>>();
    let mut dists = vec![i32::MAX; adj.len()];
    for &(i, _) in &queue {
        dists[i] = 0;
    }
    while let Some((idx, dist)) = queue.pop_front() {
        let next_dist = dist + 1;
        for &nbr_idx in &adj[idx] {
            if let Some(d) = dists.get(nbr_idx)
                && *d > next_dist
                && map.node_at_idx(nbr_idx).unwrap().cost < 10
            {
                dists[nbr_idx] = next_dist;
                queue.push_back((nbr_idx, next_dist));
            }
        }
    }
    dists
}

fn custom_distances(
    map: &HexMap,
    adj: &[[usize; 6]],
    finish_board_idx: u8,
    cost_fn: impl Fn(&Node) -> f64,
) -> Vec<f64> {
    // Min-heap element.
    #[derive(PartialEq)]
    struct MinElem {
        cost: f64,
        idx: usize,
    }
    impl Eq for MinElem {}
    impl PartialOrd for MinElem {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for MinElem {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other
                .cost
                .partial_cmp(&self.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }
    // Dijkstra's algorithm.
    let mut queue = BinaryHeap::<MinElem>::new();
    let mut dists = vec![f64::INFINITY; adj.len()];
    // Search backwards from the finish nodes.
    for (i, (_, node)) in map.all_nodes().enumerate() {
        if node.board_idx == finish_board_idx {
            queue.push(MinElem { cost: 0.0, idx: i });
            dists[i] = 0.0;
        }
    }
    while let Some(MinElem { cost, idx }) = queue.pop() {
        if cost > dists[idx] {
            continue;
        }
        let node = map.node_at_idx(idx).unwrap();
        let next_cost = cost + cost_fn(node);
        for &nbr_idx in &adj[idx] {
            if let Some(d) = dists.get(nbr_idx)
                && next_cost < *d
                && map.node_at_idx(nbr_idx).unwrap().cost < 10
            {
                dists[nbr_idx] = next_cost;
                queue.push(MinElem {
                    cost: next_cost,
                    idx: nbr_idx,
                });
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
