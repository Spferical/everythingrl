use std::collections::HashSet;

use crate::Pos;

pub fn dfs(starts: &[Pos], reachable: impl FnMut(Pos) -> Vec<Pos>) -> impl Iterator<Item = Pos> {
    Dfs {
        stack: starts.to_vec(),
        visited: starts.iter().cloned().collect::<HashSet<_>>(),
        reachable,
        to_emit: starts.to_vec(),
    }
}

struct Dfs<R: FnMut(Pos) -> Vec<Pos>> {
    stack: Vec<Pos>,
    visited: HashSet<Pos>,
    reachable: R,
    to_emit: Vec<Pos>,
}

impl<R: FnMut(Pos) -> Vec<Pos>> Iterator for Dfs<R> {
    type Item = Pos;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(p) = self.to_emit.pop() {
                return Some(p);
            }
            if let Some(p) = self.stack.pop() {
                let mut reachable = (self.reachable)(p);
                reachable.retain(|p| !self.visited.contains(p));
                self.visited.extend(reachable.iter());
                self.stack.extend(reachable.iter());
                self.to_emit.extend(reachable);
            } else {
                return None;
            }
        }
    }
}

pub fn bfs_paths(
    starts: &[Pos],
    maxdist: usize,
    reachable: impl FnMut(Pos) -> Vec<Pos>,
) -> impl Iterator<Item = Vec<Pos>> {
    Bfs {
        periphery: starts.iter().map(|p| vec![*p]).collect(),
        new_periphery: vec![],
        visited: starts.iter().cloned().collect::<HashSet<_>>(),
        reachable,
        to_emit: starts.iter().map(|p| vec![*p]).collect(),
        maxdist,
    }
}

struct Bfs<R: FnMut(Pos) -> Vec<Pos>> {
    periphery: Vec<Vec<Pos>>,
    new_periphery: Vec<Vec<Pos>>,
    visited: HashSet<Pos>,
    reachable: R,
    to_emit: Vec<Vec<Pos>>,
    maxdist: usize,
}

impl<R: FnMut(Pos) -> Vec<Pos>> Iterator for Bfs<R> {
    type Item = Vec<Pos>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(path) = self.to_emit.pop() {
                return Some(path);
            }
            if let Some(mut path) = self.periphery.pop() {
                let mut reachable = (self.reachable)(*path.last().unwrap());
                reachable.retain(|p| !self.visited.contains(p));
                self.visited.extend(reachable.iter());
                for pos in reachable {
                    path.push(pos);
                    self.to_emit.push(path.clone());
                    if path.len() < self.maxdist {
                        self.new_periphery.push(path.clone());
                    }
                    path.pop();
                }
            } else if !self.new_periphery.is_empty() {
                std::mem::swap(&mut self.periphery, &mut self.new_periphery);
            } else {
                return None;
            }
        }
    }
}
