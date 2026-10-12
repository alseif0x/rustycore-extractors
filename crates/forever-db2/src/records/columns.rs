// SPDX-License-Identifier: GPL-3.0-or-later
use super::RecordError;
use crate::Header;
use std::collections::BTreeMap;
pub(super) struct Column {
    pub offset: u16,
    pub compression: u32,
    extra: usize,
    a: u32,
    b: u32,
    c: u32,
    pallet: Vec<u32>,
    common: BTreeMap<u32, u32>,
}
/// Reads the 244-byte header/section prefix, the field offset table and every
/// 24-byte column entry, then the pallet and common blocks. Every size is
/// checked before it is used as a slice bound so no arithmetic can overflow.
pub(super) fn columns(data: &[u8], h: &Header) -> Result<(Vec<Column>, usize), RecordError> {
    columns_after_prefix(data, h, 244)
}
/// The explicit regular multi-section API supplies its checked header/section
/// prefix. Legacy callers retain the fixed 244-byte single-section prefix.
pub(super) fn columns_after_prefix(
    data: &[u8],
    h: &Header,
    prefix: usize,
) -> Result<(Vec<Column>, usize), RecordError> {
    let fields = h.field_count as usize;
    let meta_bytes = fields.checked_mul(24).ok_or(RecordError::Column)?;
    if h.column_meta_size as usize != meta_bytes {
        return Err(RecordError::Column);
    }
    let offsets_bytes = fields.checked_mul(4).ok_or(RecordError::Bounds)?;
    let start = prefix
        .checked_add(offsets_bytes)
        .ok_or(RecordError::Bounds)?;
    let mut at = start.checked_add(meta_bytes).ok_or(RecordError::Bounds)?;
    if at > data.len() {
        return Err(RecordError::Bounds);
    }
    let mut cols = Vec::with_capacity(fields);
    for field in 0..fields {
        let offset = field
            .checked_mul(24)
            .and_then(|value| start.checked_add(value))
            .ok_or(RecordError::Bounds)?;
        let entry = data
            .get(offset..offset.checked_add(24).ok_or(RecordError::Bounds)?)
            .ok_or(RecordError::Bounds)?;
        let compression = u32::from_le_bytes(entry[8..12].try_into().unwrap());
        if compression > 5 {
            return Err(RecordError::Column);
        }
        cols.push(Column {
            offset: u16::from_le_bytes(entry[0..2].try_into().unwrap()),
            compression,
            extra: u32::from_le_bytes(entry[4..8].try_into().unwrap()) as usize,
            a: u32::from_le_bytes(entry[12..16].try_into().unwrap()),
            b: u32::from_le_bytes(entry[16..20].try_into().unwrap()),
            c: u32::from_le_bytes(entry[20..24].try_into().unwrap()),
            pallet: Vec::new(),
            common: BTreeMap::new(),
        });
    }
    let pallet_start = at;
    // Source stores all scalar pallets, then all array pallets, then common data.
    for mode in [3u32, 4, 2] {
        for col in &mut cols {
            if col.compression != mode {
                continue;
            }
            let stride = if mode == 2 { 8 } else { 4 };
            if col.extra % stride != 0 {
                return Err(RecordError::Column);
            }
            let end = at.checked_add(col.extra).ok_or(RecordError::Bounds)?;
            if end > data.len() {
                return Err(RecordError::Bounds);
            }
            for pos in (at..end).step_by(stride) {
                let key_end = pos.checked_add(4).ok_or(RecordError::Bounds)?;
                let key = u32::from_le_bytes(
                    data.get(pos..key_end)
                        .ok_or(RecordError::Bounds)?
                        .try_into()
                        .unwrap(),
                );
                if mode == 2 {
                    let value_end = key_end.checked_add(4).ok_or(RecordError::Bounds)?;
                    let value = u32::from_le_bytes(
                        data.get(key_end..value_end)
                            .ok_or(RecordError::Bounds)?
                            .try_into()
                            .unwrap(),
                    );
                    col.common.insert(key, value);
                } else {
                    col.pallet.push(key);
                }
            }
            at = end;
        }
        if mode == 4 {
            let consumed = at.checked_sub(pallet_start).ok_or(RecordError::Bounds)?;
            if consumed != h.pallet_data_size as usize {
                return Err(RecordError::Column);
            }
        }
    }
    let consumed = at.checked_sub(pallet_start).ok_or(RecordError::Bounds)?;
    let total_extra = (h.pallet_data_size as usize)
        .checked_add(h.common_data_size as usize)
        .ok_or(RecordError::Bounds)?;
    if consumed != total_extra {
        return Err(RecordError::Column);
    }
    for col in &cols {
        if matches!(col.compression, 0 | 1 | 5) && col.extra != 0 {
            return Err(RecordError::Column);
        }
    }
    Ok((cols, at))
}
impl Column {
    /// Returns the raw masked value. The width may be up to 64 bits: compression
    /// 0 reads `width / 8` little-endian bytes into a `u64`, so an unsigned
    /// 64-bit (`FT_LONG`) field is returned whole. Packed/pallet modes stay
    /// bounded to 32 bits because their storage is a single 32-bit word.
    pub fn bits(
        &self,
        record: &[u8],
        packed: u32,
        id: u32,
        array: usize,
        width: u32,
        count: usize,
    ) -> Result<u64, RecordError> {
        if array >= count || width == 0 || width > 64 {
            return Err(RecordError::Column);
        }
        let mask = if width == 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        let value = match self.compression {
            0 => {
                // Direct-value storage is only defined for whole-byte widths.
                // A non-multiple-of-8 width would otherwise read zero bytes and
                // silently produce a wrong value, so reject it explicitly.
                if !self.offset.is_multiple_of(8) || !width.is_multiple_of(8) {
                    return Err(RecordError::Column);
                }
                let byte_width = (width / 8) as usize;
                let at = (self.offset as usize / 8)
                    .checked_add(array.checked_mul(byte_width).ok_or(RecordError::Column)?)
                    .ok_or(RecordError::Bounds)?;
                let bytes = record
                    .get(at..at.checked_add(byte_width).ok_or(RecordError::Bounds)?)
                    .ok_or(RecordError::Bounds)?;
                let mut word = [0u8; 8];
                word[..byte_width].copy_from_slice(bytes);
                u64::from_le_bytes(word)
            }
            2 => {
                // Pinned RecordGetVarInt::CommonData copies at most four
                // bytes into an uninitialized T. A 64-bit T has no defined
                // upper half; reject for signed and unsigned fields alike.
                if width > 32 || count != 1 {
                    return Err(RecordError::Column);
                }
                u64::from(*self.common.get(&id).unwrap_or(&self.a))
            }
            mode => {
                // `b` and the bit accumulator are 32-bit: a width above 32 has
                // no defined packed storage here, so reject rather than shift
                // out of range.
                if width > 32 || self.b == 0 || self.b > 32 || (mode != 4 && count != 1) {
                    return Err(RecordError::Column);
                }
                let bit = (packed as usize)
                    .checked_mul(8)
                    .and_then(|bit| bit.checked_add(self.a as usize))
                    .ok_or(RecordError::Bounds)?;
                let end = bit
                    .checked_add(self.b as usize)
                    .ok_or(RecordError::Bounds)?;
                let record_bits = record.len().checked_mul(8).ok_or(RecordError::Bounds)?;
                if end > record_bits {
                    return Err(RecordError::Bounds);
                }
                let mut value = 0u32;
                for index in 0..self.b as usize {
                    let pos = bit.checked_add(index).ok_or(RecordError::Bounds)?;
                    let byte = *record.get(pos / 8).ok_or(RecordError::Bounds)?;
                    value |= u32::from((byte >> (pos % 8)) & 1) << index;
                }
                match mode {
                    1 => u64::from(value),
                    5 => {
                        let sign = 1u32 << (self.b - 1);
                        u64::from((value ^ sign).wrapping_sub(sign))
                    }
                    3 => u64::from(*self.pallet.get(value as usize).ok_or(RecordError::Column)?),
                    4 => {
                        if self.c as usize != count {
                            return Err(RecordError::Column);
                        }
                        let index = (value as usize)
                            .checked_mul(count)
                            .and_then(|index| index.checked_add(array))
                            .ok_or(RecordError::Column)?;
                        u64::from(*self.pallet.get(index).ok_or(RecordError::Column)?)
                    }
                    _ => return Err(RecordError::Column),
                }
            }
        };
        Ok(value & mask)
    }
}
#[cfg(test)]
mod tests {
    use super::Column;
    use crate::records::RecordError;
    use std::collections::BTreeMap;
    fn direct(offset: u16) -> Column {
        Column {
            offset,
            compression: 0,
            extra: 0,
            a: 0,
            b: 0,
            c: 0,
            pallet: Vec::new(),
            common: BTreeMap::new(),
        }
    }
    #[test]
    fn wide_unsigned_reads_eight_little_endian_bytes() {
        // compression 0 at offset 0 reads `width / 8` whole bytes: a 64-bit
        // field must return all eight bytes and never a 32-bit truncation.
        let mut record = [0u8; 16];
        record[..8].copy_from_slice(&0x1_0000_0001u64.to_le_bytes());
        let value = direct(0).bits(&record, 0, 0, 0, 64, 1).unwrap();
        assert_eq!(value, 0x1_0000_0001);
    }
    #[test]
    fn direct_thirty_three_bit_width_is_rejected() {
        // A non-multiple-of-8 width has no defined direct storage, so it is a
        // column error instead of silently reading an empty slice.
        let record = [0u8; 8];
        assert_eq!(
            direct(0).bits(&record, 0, 0, 0, 33, 1),
            Err(RecordError::Column)
        );
    }
    #[test]
    fn direct_offset_must_be_a_multiple_of_eight() {
        // A byte-aligned read starts at `offset / 8`, so an offset that is not
        // a multiple of eight would point into the middle of a word.
        let record = [0u8; 8];
        assert_eq!(
            direct(4).bits(&record, 0, 0, 0, 32, 1),
            Err(RecordError::Column)
        );
    }

    #[test]
    fn common_data_cannot_define_upper_half_of_64_bit_field() {
        let mut column = direct(0);
        column.compression = 2;
        column.a = u32::MAX;
        column.common.insert(7, 0x12345678);
        for id in [7, 8] {
            assert_eq!(column.bits(&[], 0, id, 0, 64, 1), Err(RecordError::Column));
        }
        assert_eq!(column.bits(&[], 0, 7, 0, 32, 1), Ok(0x12345678));
    }
}
