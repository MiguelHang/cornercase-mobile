use std::cmp::Ordering;
use std::collections::HashMap;
use std::mem;

use super::rank;
use crate::cli::status::Pane;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Item<'a> {
    pub(crate) pane: &'a Pane,
    pub(crate) since: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rank<'a> {
    pub(crate) status: Option<&'a str>,
    pub(crate) since: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node<'a> {
    pub(crate) at: usize,
    pub(crate) depth: u32,
    pub(crate) parent: Option<u64>,
    pub(crate) rank: Rank<'a>,
}

impl<'a> Rank<'a> {
    fn of(item: &Item<'a>) -> Self {
        Self { status: item.pane.status.as_deref(), since: item.since }
    }

    fn raises(self) -> bool {
        matches!(self.status, Some("waiting" | "failed"))
    }

    fn first(self, other: Self) -> Ordering {
        rank(other.status).cmp(&rank(self.status)).then_with(|| self.since.cmp(&other.since))
    }

    fn louder(self, other: Self) -> Self {
        if other.first(self).is_lt() { other } else { self }
    }
}

struct Branch<'a> {
    at: usize,
    pane: u64,
    rank: Rank<'a>,
    raised: Option<Rank<'a>>,
    children: Vec<Branch<'a>>,
}

impl<'a> Branch<'a> {
    fn new(at: usize, item: &Item<'a>, mut children: Vec<Branch<'a>>) -> Self {
        children.sort_by(Branch::first);
        let own = Rank::of(item);
        let raising = children.iter().filter_map(|child| child.raised);
        let raised = raising.chain(own.raises().then_some(own)).reduce(Rank::louder);
        let rank = raised.map_or(own, |raised| own.louder(raised));
        Self { at, pane: item.pane.id, rank, raised, children }
    }

    fn first(&self, other: &Self) -> Ordering {
        self.rank.first(other.rank).then_with(|| self.at.cmp(&other.at))
    }

    fn emit(self, depth: u32, parent: Option<u64>, nodes: &mut Vec<Node<'a>>) {
        nodes.push(Node { at: self.at, depth, parent, rank: self.rank });
        for child in self.children {
            child.emit(depth + 1, Some(self.pane), nodes);
        }
    }
}

struct Walk<'a, 'b> {
    items: &'b [Item<'a>],
    kids: Vec<Vec<usize>>,
    visited: Vec<bool>,
}

impl<'a> Walk<'a, '_> {
    fn grow(&mut self, at: usize) -> Option<Branch<'a>> {
        if mem::replace(&mut self.visited[at], true) {
            return None;
        }
        let kids = mem::take(&mut self.kids[at]);
        let children = kids.into_iter().filter_map(|kid| self.grow(kid)).collect();
        Some(Branch::new(at, &self.items[at], children))
    }
}

pub(crate) fn order<'a>(items: &[Item<'a>]) -> Vec<Node<'a>> {
    let (kids, roots) = family(items);
    let mut walk = Walk { items, kids, visited: vec![false; items.len()] };
    let mut branches: Vec<Branch<'a>> =
        roots.into_iter().chain(0..items.len()).filter_map(|at| walk.grow(at)).collect();
    branches.sort_by(Branch::first);
    let mut nodes = Vec::with_capacity(items.len());
    for branch in branches {
        branch.emit(0, None, &mut nodes);
    }
    nodes
}

fn family(items: &[Item]) -> (Vec<Vec<usize>>, Vec<usize>) {
    let listed: HashMap<u64, usize> = items.iter().enumerate().map(|(at, item)| (item.pane.id, at)).collect();
    let mut kids = vec![Vec::new(); items.len()];
    let mut roots = Vec::new();
    for (at, item) in items.iter().enumerate() {
        match item.pane.parent.and_then(|parent| listed.get(&parent)) {
            Some(parent) => kids[*parent].push(at),
            None => roots.push(at),
        }
    }
    (kids, roots)
}

#[cfg(test)]
mod tests;
