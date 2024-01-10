use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use crate::Position;

#[derive(Clone, Eq, PartialEq)]
struct State {
    path: Vec<Position>,
    position: Position,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .path
            .len()
            .cmp(&self.path.len())
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Dijkstra {
    cache: HashMap<(Position, Position), Vec<Position>>,
}

impl Dijkstra {
    pub fn new() -> Dijkstra {
        Dijkstra {
            cache: HashMap::new(),
        }
    }

    pub fn shortest_path(
        &mut self,
        neighbors: &HashMap<Position, Vec<Position>>,
        start: Position,
        target: Position,
    ) -> Vec<Position> {
        if let Some(path) = self.cache.get(&(start, target)) {
            return path.clone();
        }

        let mut dist: HashMap<Position, usize> =
            neighbors.keys().map(|k| (*k, usize::MAX)).collect();

        let mut heap = BinaryHeap::new();

        *dist.get_mut(&start).unwrap() = 0;
        heap.push(
            State {
                path: vec![],
                position: start,
            }
            .clone(),
        );

        while let Some(State { path, position }) = heap.pop() {
            if position == target {
                self.cache.insert((start, target), path.clone());
                return path;
            }

            if path.len() > *dist.get(&position).unwrap() {
                continue;
            }

            for n_pos in neighbors.get(&position).unwrap() {
                let mut path = path.clone();
                path.push(*n_pos);
                let next = State {
                    path,
                    position: *n_pos,
                };

                if next.path.len() < *dist.get(n_pos).unwrap() {
                    *dist.get_mut(n_pos).unwrap() = next.path.len();
                    heap.push(next);
                }
            }
        }
        vec![]
    }
}
