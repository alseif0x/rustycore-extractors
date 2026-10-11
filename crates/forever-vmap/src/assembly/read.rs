// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded readers and explicit placement-proof associations.
use super::*;
pub struct Tree<'a> {
    bytes: &'a [u8],
    bih: bih::View<'a>,
}
impl<'a> Tree<'a> {
    /// Complete intrinsic-count validation, without an external placement
    /// proof. Geometry/placement binding remains the store loader's duty.
    pub fn parse_intrinsic(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_output_bytes {
            return Err(Error::Limit);
        }
        if bytes.get(..8) != Some(MAGIC.as_slice()) || bytes.get(8..12) != Some(b"NODE".as_slice())
        {
            return Err(Error::Framing);
        }
        let mut tree_limits = limits.tree(0)?;
        tree_limits.max_primitives = tree_limits.max_primitives.min(limits.max_spawns);
        let bih = bih::View::parse_intrinsic(bytes.get(12..).ok_or(Error::Framing)?, tree_limits)?;
        Ok(Self { bytes, bih })
    }
    pub fn parse(bytes: &'a [u8], expected_spawns: usize, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_output_bytes || expected_spawns > limits.max_spawns {
            return Err(Error::Limit);
        }
        if bytes.get(..8) != Some(MAGIC.as_slice()) || bytes.get(8..12) != Some(b"NODE".as_slice())
        {
            return Err(Error::Framing);
        }
        let bih = bih::View::parse(
            bytes.get(12..).ok_or(Error::Framing)?,
            expected_spawns,
            limits.tree(0)?,
        )?;
        Ok(Self { bytes, bih })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn bih(&self) -> &bih::View<'a> {
        &self.bih
    }
}
pub struct TileView<'a> {
    bytes: &'a [u8],
    records: placement::DirBin<'a>,
}
impl<'a> TileView<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let count = header(bytes, limits)?;
        let records = placement::DirBin::parse(&bytes[12..], limits.codec(limits.max_tile_bytes))?;
        if records.record_count() != count || count == 0 {
            return Err(Error::Framing);
        }
        let mut last = None;
        let mut own = false;
        for p in records.records() {
            if p.bounds_bits().is_none() {
                return Err(Error::MissingBounds);
            }
            let parent = p.flags() & 2 != 0;
            own |= !parent;
            let key = (parent, p.id());
            if last.is_some_and(|v| v >= key) {
                return Err(Error::Order);
            }
            last = Some(key);
        }
        if !own {
            return Err(Error::Order);
        }
        // Each class is sorted, but the same ID in both classes is not a native
        // fresh-map output. A two-stream merge checks this without allocation.
        let mut own_ids = records.records().filter(|p| p.flags() & 2 == 0).peekable();
        let mut parent_ids = records.records().filter(|p| p.flags() & 2 != 0).peekable();
        while let (Some(a), Some(b)) = (own_ids.peek(), parent_ids.peek()) {
            match a.id().cmp(&b.id()) {
                std::cmp::Ordering::Less => {
                    own_ids.next();
                }
                std::cmp::Ordering::Greater => {
                    parent_ids.next();
                }
                std::cmp::Ordering::Equal => return Err(Error::Order),
            }
        }
        Ok(Self { bytes, records })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn records(&self) -> placement::Records<'a> {
        self.records.records()
    }
    pub const fn count(&self) -> usize {
        self.records.record_count()
    }
}
pub struct IndexView<'a> {
    bytes: &'a [u8],
    count: usize,
    peak: usize,
}
impl<'a> IndexView<'a> {
    pub fn parse(bytes: &'a [u8], expected_spawns: usize, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if expected_spawns > limits.max_spawns {
            return Err(Error::Limit);
        }
        let count = header(bytes, limits)?;
        if count == 0 || count > expected_spawns || bytes.len() != add(12, mul(count, 4)?)? {
            return Err(Error::Framing);
        }
        let mut memory = Memory::new(limits.max_memory_bytes);
        // Scratch depends on this tile's count, not the entire map. A map with
        // many singleton tiles must not clear a map-sized bitmap for each tile.
        let mut seen = Vec::new();
        memory.reserve(&mut seen, count)?;
        for value in bytes[12..].as_chunks::<4>().0 {
            let ordinal = u32::from_le_bytes(*value);
            if ordinal as usize >= expected_spawns {
                return Err(Error::Index);
            }
            seen.push(ordinal);
        }
        seen.sort_unstable();
        if seen.windows(2).any(|p| p[0] == p[1]) {
            return Err(Error::Index);
        }
        Ok(Self {
            bytes,
            count,
            peak: memory.peak,
        })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn count(&self) -> usize {
        self.count
    }
    pub const fn validation_peak_bytes(&self) -> usize {
        self.peak
    }
    pub fn indices(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.bytes[12..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|v| u32::from_le_bytes(*v))
    }
}
fn header(bytes: &[u8], limits: Limits) -> Result<usize, Error> {
    if bytes.len() > limits.max_tile_bytes {
        return Err(Error::Limit);
    }
    if bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err(Error::Framing);
    }
    let count = word(bytes, 8)? as usize;
    if count > limits.max_spawns || count > limits.max_associations {
        return Err(Error::Limit);
    }
    Ok(count)
}
fn proof(bytes: &[u8], limits: Limits) -> Result<placement::DirBin<'_>, Error> {
    let records = placement::DirBin::parse(bytes, limits.codec(limits.max_input_bytes))?;
    let mut last = None;
    for p in records.records() {
        rectangle(&p)?;
        if last.is_some_and(|id| id >= p.id()) {
            return Err(Error::Order);
        }
        last = Some(p.id());
    }
    Ok(records)
}
/// Check owned/mixed payload and child index against complete ordered records.
/// This is correspondence proof, not authentication, tile membership, inventory
/// completeness, or a comparison of BIH bounds with those records.
pub fn verify_associations(
    payload: &[u8],
    index: &[u8],
    ordered: &[u8],
    limits: Limits,
) -> Result<(), Error> {
    verify(payload, index, ordered, limits, false)
}
/// Parent-only child output needs an ACTUAL ancestor payload. Every selected
/// child record must carry parent bit 2; all other bytes must match exactly in
/// the supplied payload order. No ancestor sorting, lookup or repair occurs.
pub fn verify_inherited_associations(
    ancestor_payload: &[u8],
    child_index: &[u8],
    child_ordered: &[u8],
    limits: Limits,
) -> Result<(), Error> {
    verify(ancestor_payload, child_index, child_ordered, limits, true)
}
fn verify(
    payload: &[u8],
    index: &[u8],
    ordered: &[u8],
    limits: Limits,
    inherited: bool,
) -> Result<(), Error> {
    limits.validate()?;
    let proof = proof(ordered, limits)?;
    let tile = TileView::parse(payload, limits)?;
    let index = IndexView::parse(index, proof.record_count(), limits)?;
    if tile.count() != index.count() {
        return Err(Error::Association);
    }
    // At most two forward passes through the proof: own sorted ordinals followed
    // by parent sorted ordinals. No random-access scratch or quadratic nth scans.
    let mut source = proof.records();
    let mut source_ordinal = 0usize;
    let mut source_at = 0usize;
    let mut payload_at = 12usize;
    let mut phase = false;
    let mut previous = None;
    for (p, ordinal) in tile.records().zip(index.indices()) {
        let parent = p.flags() & 2 != 0;
        if !inherited && parent && !phase {
            source = proof.records();
            source_ordinal = 0;
            source_at = 0;
            previous = None;
            phase = true;
        }
        let ordinal = ordinal as usize;
        if previous.is_some_and(|old| old >= ordinal) {
            return Err(Error::Order);
        }
        previous = Some(ordinal);
        let selected = loop {
            let record = source.next().ok_or(Error::Index)?;
            let start = source_at;
            source_at = add(source_at, encoded_len(&record)?)?;
            let current = source_ordinal;
            source_ordinal += 1;
            if current == ordinal {
                break (record, start, source_at);
            }
        };
        let end = add(payload_at, encoded_len(&p)?)?;
        let got = payload.get(payload_at..end).ok_or(Error::Framing)?;
        let expected = ordered.get(selected.1..selected.2).ok_or(Error::Framing)?;
        if inherited {
            if selected.0.flags() & 2 == 0
                || got.len() != expected.len()
                || got[0] & !2 != expected[0] & !2
                || got[1..] != expected[1..]
            {
                return Err(Error::Association);
            }
        } else if got != expected {
            return Err(Error::Association);
        }
        payload_at = end;
    }
    Ok(())
}
