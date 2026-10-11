// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: provider-neutral, capacity-bounded artifact transfer.
use super::{
    Error, Limits,
    memory::{Memory, Work, add},
};

/// Exact final-artifact identity, without path normalization or filename guesses.
/// One Source represents one immutable package; model names are exact bytes.
#[derive(Clone, Copy)]
pub enum Artifact<'a> {
    Tree { map_id: u32 },
    Tile { map_id: u32, x: u16, y: u8 },
    Index { map_id: u32, x: u16, y: u8 },
    Model { name: &'a [u8] },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Missing,
    Present,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Io,
    InputChanged,
    Limit,
}
/// Providers must return the complete file, including exact EOF and their
/// identity checks. Only genuine absence is Missing. Any previously observed
/// artifact/inventory mutation must be InputChanged, never Missing or fallback.
/// Provider scratch and identity inventory require a separate bounded policy.
pub trait Source {
    fn read(
        &mut self,
        artifact: Artifact<'_>,
        output: &mut ReadBuffer<'_>,
    ) -> Result<Presence, SourceError>;
    fn verify_unchanged(&mut self) -> Result<(), SourceError>;
}
/// Store-owned transfer buffer. Growth is admitted before allocation, including
/// old/new capacity overlap; a provider cannot return an unbudgeted huge Vec.
pub struct ReadBuffer<'a> {
    bytes: Vec<u8>,
    limit: usize,
    memory: &'a mut Memory,
}
impl ReadBuffer<'_> {
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), SourceError> {
        let wanted = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(SourceError::Limit)?;
        if wanted > self.limit {
            return Err(SourceError::Limit);
        }
        if wanted > self.bytes.capacity() {
            let growth = wanted
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limit);
            self.memory
                .reserve(&mut self.bytes, growth)
                .map_err(|_| SourceError::Limit)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub const fn limit(&self) -> usize {
        self.limit
    }
}
pub(super) fn read<S: Source>(
    source: &mut S,
    artifact: Artifact<'_>,
    limits: Limits,
    memory: &mut Memory,
    work: &mut Work,
) -> Result<Option<Vec<u8>>, Error> {
    work.charge(1)?;
    work.reads = work.reads.checked_add(1).ok_or(Error::Limit)?;
    let mut buffer = ReadBuffer {
        bytes: Vec::new(),
        limit: limits.max_file_bytes.min(work.remaining_bytes()),
        memory,
    };
    let result = source.read(artifact, &mut buffer);
    work.bytes = add(work.bytes, buffer.len())?;
    let bytes = buffer.bytes;
    match result? {
        Presence::Missing => {
            if !bytes.is_empty() {
                return Err(Error::Source(SourceError::Io));
            }
            memory.release(bytes.capacity());
            Ok(None)
        }
        Presence::Present => Ok(Some(bytes)),
    }
}
