// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded GNU libstdc++ 15 traversal for the separately selected collector.
//!
//! This models default unordered_set<u16> with identity hashing, load factor
//! one and individual insertions. It does not change strict sorted selections.
//! Source reference: GCC fa37bfd7f2d01858323d7b8100a154d0a526b8f9,
//! bits/hashtable.h and src/c++11/hashtable_c++0x.cc. The native comparison's
//! original headers match that snapshot. Other library policies are not implied.
use crate::collision::Error;

const NONE: usize = usize::MAX;
const HEAD: usize = usize::MAX - 1;
const MAX_REFERENCES: usize = 65536;
// The default policy's single-insertion growth sequence within the u16 domain.
const BUCKETS: [usize; 13] = [
    13, 29, 59, 127, 257, 541, 1109, 2357, 5087, 10273, 20753, 42043, 85229,
];

struct Node {
    value: u16,
    next: usize,
}

/// All three vector ceilings are reserved before traversal. Rehash uses the
/// existing bucket storage and rewires nodes without allocating or copying them.
pub(super) struct Order {
    buckets: Vec<usize>,
    nodes: Vec<Node>,
    output: Vec<u16>,
    head: usize,
    bucket_count: usize,
    expected: usize,
}

impl Order {
    pub(super) fn new(expected: usize, max_memory_bytes: usize) -> Result<Self, Error> {
        if expected > MAX_REFERENCES {
            return Err(Error::Limit);
        }
        let bucket_capacity = if expected == 0 {
            0
        } else {
            *BUCKETS
                .iter()
                .find(|n| **n >= expected)
                .ok_or(Error::Limit)?
        };
        let required = bucket_capacity
            .checked_mul(size_of::<usize>())
            .and_then(|n| n.checked_add(expected.checked_mul(size_of::<Node>())?))
            .and_then(|n| n.checked_add(expected.checked_mul(size_of::<u16>())?))
            .ok_or(Error::Limit)?;
        if required > max_memory_bytes {
            return Err(Error::Limit);
        }
        let mut buckets = Vec::new();
        let mut nodes = Vec::new();
        let mut output = Vec::new();
        buckets
            .try_reserve_exact(bucket_capacity)
            .map_err(|_| Error::Allocation)?;
        nodes
            .try_reserve_exact(expected)
            .map_err(|_| Error::Allocation)?;
        output
            .try_reserve_exact(expected)
            .map_err(|_| Error::Allocation)?;
        let actual = buckets
            .capacity()
            .checked_mul(size_of::<usize>())
            .and_then(|n| n.checked_add(nodes.capacity().checked_mul(size_of::<Node>())?))
            .and_then(|n| n.checked_add(output.capacity().checked_mul(size_of::<u16>())?))
            .ok_or(Error::Limit)?;
        if actual > max_memory_bytes {
            return Err(Error::Limit);
        }
        buckets.resize(bucket_capacity, NONE);
        Ok(Self {
            buckets,
            nodes,
            output,
            head: NONE,
            bucket_count: 1,
            expected,
        })
    }

    /// Charge this entire capacity until finish drops buckets and nodes. The
    /// returned reference vector's capacity then remains charged by its owner.
    pub(super) fn work_bytes(&self) -> Result<usize, Error> {
        self.buckets
            .capacity()
            .checked_mul(size_of::<usize>())
            .and_then(|n| n.checked_add(self.nodes.capacity().checked_mul(size_of::<Node>())?))
            .and_then(|n| n.checked_add(self.output.capacity().checked_mul(size_of::<u16>())?))
            .ok_or(Error::Limit)
    }

    fn next(&self, previous: usize) -> usize {
        if previous == HEAD {
            self.head
        } else {
            self.nodes[previous].next
        }
    }

    fn set_next(&mut self, previous: usize, next: usize) {
        if previous == HEAD {
            self.head = next;
        } else {
            self.nodes[previous].next = next;
        }
    }

    fn bucket(&self, value: u16) -> usize {
        usize::from(value) % self.bucket_count
    }

    fn contains(&self, value: u16) -> bool {
        let bucket = self.bucket(value);
        let previous = self.buckets[bucket];
        if previous == NONE {
            return false;
        }
        let mut at = self.next(previous);
        while at != NONE && self.bucket(self.nodes[at].value) == bucket {
            if self.nodes[at].value == value {
                return true;
            }
            at = self.nodes[at].next;
        }
        false
    }

    fn prepend(&mut self, index: usize) {
        let bucket = self.bucket(self.nodes[index].value);
        if self.buckets[bucket] == NONE {
            self.nodes[index].next = self.head;
            if self.head != NONE {
                let old_bucket = self.bucket(self.nodes[self.head].value);
                self.buckets[old_bucket] = index;
            }
            self.head = index;
            self.buckets[bucket] = HEAD;
        } else {
            let previous = self.buckets[bucket];
            self.nodes[index].next = self.next(previous);
            self.set_next(previous, index);
        }
    }

    fn grow(&mut self) -> Result<(), Error> {
        let count = *BUCKETS
            .iter()
            .find(|n| **n > self.bucket_count)
            .ok_or(Error::Limit)?;
        if count > self.buckets.len() {
            return Err(Error::State);
        }
        self.bucket_count = count;
        self.buckets[..count].fill(NONE);
        let mut at = self.head;
        self.head = NONE;
        while at != NONE {
            let next = self.nodes[at].next;
            self.prepend(at);
            at = next;
        }
        Ok(())
    }

    /// Insert only source-validated collision references, in physical MODR order.
    /// A duplicate leaves both traversal and the rehash threshold unchanged.
    pub(super) fn insert(&mut self, value: u16) -> Result<bool, Error> {
        if self.expected == 0 {
            return Err(Error::State);
        }
        if self.contains(value) {
            return Ok(false);
        }
        if self.nodes.len() >= self.expected {
            return Err(Error::State);
        }
        if self.bucket_count == 1 || self.nodes.len() == self.bucket_count {
            self.grow()?;
        }
        let index = self.nodes.len();
        self.nodes.push(Node { value, next: NONE });
        self.prepend(index);
        Ok(true)
    }

    pub(super) fn finish(mut self) -> Result<Vec<u16>, Error> {
        if self.nodes.len() != self.expected {
            return Err(Error::State);
        }
        let mut at = self.head;
        while at != NONE {
            if self.output.len() >= self.expected {
                return Err(Error::State);
            }
            let node = self.nodes.get(at).ok_or(Error::State)?;
            self.output.push(node.value);
            at = node.next;
        }
        if self.output.len() != self.expected {
            return Err(Error::State);
        }
        Ok(self.output)
    }
}

#[cfg(test)]
mod tests;
