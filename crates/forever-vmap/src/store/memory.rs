// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: capacity and per-load work accounting.
use super::{Error, Limits};
pub(super) fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
pub(super) fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
pub(super) struct Memory {
    pub live: usize,
    pub peak: usize,
    limit: usize,
}
impl Memory {
    pub fn new(base: usize, limit: usize) -> Self {
        Self {
            live: base,
            peak: base,
            limit,
        }
    }
    pub fn free(&self) -> usize {
        self.limit.saturating_sub(self.live)
    }
    pub fn observe(&mut self, extra: usize) -> Result<(), Error> {
        let next = add(self.live, extra)?;
        if next > self.limit {
            return Err(Error::Limit);
        }
        self.peak = self.peak.max(next);
        Ok(())
    }
    pub fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.observe(bytes)?;
        self.live += bytes;
        Ok(())
    }
    pub fn release(&mut self, bytes: usize) {
        self.live = self.live.checked_sub(bytes).expect("charged storage");
    }
    pub fn reserve<T>(&mut self, v: &mut Vec<T>, desired: usize) -> Result<(), Error> {
        if desired <= v.capacity() {
            return Ok(());
        }
        let old = mul(v.capacity(), size_of::<T>())?;
        self.observe(mul(desired, size_of::<T>())?)?;
        v.try_reserve_exact(desired.checked_sub(v.len()).ok_or(Error::Limit)?)
            .map_err(|_| Error::Allocation)?;
        let new = mul(v.capacity(), size_of::<T>())?;
        self.observe(new)?;
        self.live = add(self.live, new.checked_sub(old).ok_or(Error::Limit)?)?;
        Ok(())
    }
    pub fn copy<T: Copy>(&mut self, source: &[T]) -> Result<Vec<T>, Error> {
        let mut v = Vec::new();
        self.reserve(&mut v, source.len())?;
        v.extend_from_slice(source);
        Ok(v)
    }
}
pub(super) struct Work {
    pub reads: u64,
    pub bytes: usize,
    pub used: u64,
    cap: u64,
    byte_cap: usize,
}
impl Work {
    pub fn new(l: Limits) -> Self {
        Self {
            reads: 0,
            bytes: 0,
            used: 0,
            cap: l.max_load_work,
            byte_cap: l.max_load_bytes,
        }
    }
    pub fn charge(&mut self, n: usize) -> Result<(), Error> {
        let n = u64::try_from(n).map_err(|_| Error::WorkLimit)?;
        let next = self.used.checked_add(n).ok_or(Error::WorkLimit)?;
        if next > self.cap {
            return Err(Error::WorkLimit);
        }
        self.used = next;
        Ok(())
    }
    pub fn remaining_bytes(&self) -> usize {
        self.byte_cap - self.bytes
    }
}
