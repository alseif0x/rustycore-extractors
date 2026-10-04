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
    let fields = h.field_count as usize;
    let meta_bytes = fields.checked_mul(24).ok_or(RecordError::Column)?;
    if h.column_meta_size as usize != meta_bytes {
        return Err(RecordError::Column);
    }
    let offsets_bytes = fields.checked_mul(4).ok_or(RecordError::Bounds)?;
    let start = 244usize
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
    pub fn bits(
        &self,
        record: &[u8],
        packed: u32,
        id: u32,
        array: usize,
        width: u32,
        count: usize,
    ) -> Result<u32, RecordError> {
        if array >= count || width == 0 || width > 32 {
            return Err(RecordError::Column);
        }
        let mask = if width == 32 {
            u32::MAX
        } else {
            (1u32 << width) - 1
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
                let mut word = [0u8; 4];
                word[..byte_width].copy_from_slice(bytes);
                u32::from_le_bytes(word)
            }
            2 => {
                if count != 1 {
                    return Err(RecordError::Column);
                }
                *self.common.get(&id).unwrap_or(&self.a)
            }
            mode => {
                if self.b == 0 || self.b > 32 || (mode != 4 && count != 1) {
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
                    1 => value,
                    5 => {
                        let sign = 1u32 << (self.b - 1);
                        (value ^ sign).wrapping_sub(sign)
                    }
                    3 => *self.pallet.get(value as usize).ok_or(RecordError::Column)?,
                    4 => {
                        if self.c as usize != count {
                            return Err(RecordError::Column);
                        }
                        let index = (value as usize)
                            .checked_mul(count)
                            .and_then(|index| index.checked_add(array))
                            .ok_or(RecordError::Column)?;
                        *self.pallet.get(index).ok_or(RecordError::Column)?
                    }
                    _ => return Err(RecordError::Column),
                }
            }
        };
        Ok(value & mask)
    }
}
