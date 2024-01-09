use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use crate::Position;

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: usize,
    position: Position,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Dijkstra {
    cache: HashMap<(Position, Position), usize>,
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
    ) -> usize {
        if let Some(dist) = self.cache.get(&(start, target)) {
            return *dist;
        }

        let mut dist: HashMap<Position, usize> =
            neighbors.keys().map(|k| (*k, usize::MAX)).collect();

        let mut heap = BinaryHeap::new();

        *dist.get_mut(&start).unwrap() = 0;
        heap.push(State {
            cost: 0,
            position: start,
        });

        while let Some(State { cost, position }) = heap.pop() {
            if position == target {
                self.cache.insert((start, target), cost);
                return cost;
            }

            if cost > *dist.get(&position).unwrap() {
                continue;
            }

            for n_pos in neighbors.get(&position).unwrap() {
                let next = State {
                    cost: cost + 1,
                    position: *n_pos,
                };

                if next.cost < *dist.get(n_pos).unwrap() {
                    heap.push(next);
                    *dist.get_mut(n_pos).unwrap() = next.cost;
                }
            }
        }
        usize::MAX
    }
}
