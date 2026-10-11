// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: strict, complete dir_bin codec; no I/O.
use super::{Error, Limits, Placement, vector, word};

/// Complete concatenated records with no magic/count/padding. No class is
/// inferred from flags: bounds can also occur on an assembled M2 placement.
pub struct DirBin<'a> {
    bytes: &'a [u8],
    count: usize,
    limits: Limits,
}
impl<'a> DirBin<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_bytes {
            return Err(Error::Limit);
        }
        let mut rest = bytes;
        let mut count = 0;
        while !rest.is_empty() {
            if count >= limits.max_records {
                return Err(Error::Limit);
            }
            let (_, size) = one(rest, limits)?;
            rest = &rest[size..];
            count += 1;
        }
        Ok(Self {
            bytes,
            count,
            limits,
        })
    }
    pub const fn record_count(&self) -> usize {
        self.count
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn records(&self) -> Records<'a> {
        Records {
            rest: self.bytes,
            remaining: self.count,
            limits: self.limits,
        }
    }
}
pub struct Records<'a> {
    rest: &'a [u8],
    remaining: usize,
    limits: Limits,
}
impl<'a> Iterator for Records<'a> {
    type Item = Placement<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let (record, size) = one(self.rest, self.limits).expect("validated dir_bin");
        self.rest = &self.rest[size..];
        self.remaining -= 1;
        Some(record)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}
impl ExactSizeIterator for Records<'_> {}
impl std::iter::FusedIterator for Records<'_> {}

fn one(bytes: &[u8], limits: Limits) -> Result<(Placement<'_>, usize), Error> {
    let flags = *bytes.first().ok_or(Error::Framing)?;
    if flags & !7 != 0 {
        return Err(Error::Flags);
    }
    let bound = flags & 1 != 0;
    let name_at = if bound { 58 } else { 34 };
    let name_len = word(bytes, name_at)? as usize;
    if name_len > limits.max_name_bytes {
        return Err(Error::Limit);
    }
    let end = (name_at + 4).checked_add(name_len).ok_or(Error::Framing)?;
    let name = bytes.get(name_at + 4..end).ok_or(Error::Framing)?;
    let bounds = if bound {
        let low = vector(bytes, 34)?;
        let high = vector(bytes, 46)?;
        Some([low[0], low[1], low[2], high[0], high[1], high[2]])
    } else {
        None
    };
    let result = Placement {
        flags,
        name_set: *bytes.get(1).ok_or(Error::Framing)?,
        id: word(bytes, 2)?,
        position: vector(bytes, 6)?,
        rotation: vector(bytes, 18)?,
        scale: word(bytes, 30)?,
        bounds,
        name,
    };
    result.validate(limits)?;
    Ok((result, end))
}

/// Validate every record and the full size before the single output allocation.
/// Returns no output on malformed records, limits or allocation failure.
pub fn encode(records: &[Placement<'_>], limits: Limits) -> Result<Vec<u8>, Error> {
    limits.validate()?;
    if records.len() > limits.max_records {
        return Err(Error::Limit);
    }
    let mut length = 0usize;
    for record in records {
        record.validate(limits)?;
        length = length
            .checked_add(record.encoded_len()?)
            .ok_or(Error::Limit)?;
        if length > limits.max_bytes {
            return Err(Error::Limit);
        }
    }
    let mut out = Vec::new();
    out.try_reserve_exact(length)
        .map_err(|_| Error::Allocation)?;
    if out.capacity() > limits.max_bytes {
        return Err(Error::Limit);
    }
    for record in records {
        out.extend_from_slice(&[record.flags, record.name_set]);
        append(&mut out, record.id);
        for v in record.position {
            append(&mut out, v);
        }
        for v in record.rotation {
            append(&mut out, v);
        }
        append(&mut out, record.scale);
        if let Some(bounds) = record.bounds {
            for v in bounds {
                append(&mut out, v);
            }
        }
        append(
            &mut out,
            u32::try_from(record.name.len()).map_err(|_| Error::Limit)?,
        );
        out.extend_from_slice(record.name);
    }
    Ok(out)
}
fn append(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
