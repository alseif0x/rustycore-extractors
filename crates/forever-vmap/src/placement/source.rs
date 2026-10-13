// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: exact borrowed ADT MDDF/MODF placement records.
use super::{Error, Limits, finite, short, valid_bounds, vector, word};

/// Exactly one packed 36-byte MDDF. Unknown source flags remain opaque.
#[derive(Clone, Copy)]
pub struct Mddf<'a> {
    bytes: &'a [u8],
}
impl<'a> Mddf<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.one(bytes.len())?;
        if bytes.len() != 36 {
            return Err(Error::Framing);
        }
        finite(&vector(bytes, 8)?)?;
        finite(&vector(bytes, 20)?)?;
        Ok(Self { bytes })
    }
    pub fn source_id(&self) -> u32 {
        word(self.bytes, 0).expect("validated MDDF")
    }
    pub fn unique_id(&self) -> u32 {
        word(self.bytes, 4).expect("validated MDDF")
    }
    pub fn position_bits(&self) -> [u32; 3] {
        vector(self.bytes, 8).expect("validated MDDF")
    }
    pub fn rotation_bits(&self) -> [u32; 3] {
        vector(self.bytes, 20).expect("validated MDDF")
    }
    pub fn scale(&self) -> u16 {
        short(self.bytes, 32)
    }
    pub fn flags(&self) -> u16 {
        short(self.bytes, 34)
    }
    pub fn uses_file_data_id(&self) -> bool {
        self.flags() & 0x40 != 0
    }
}
/// Exactly one packed 64-byte MODF. DoodadSet is retained for later selection;
/// no doodad expansion or validation of the set against a WMO occurs here.
#[derive(Clone, Copy)]
pub struct Modf<'a> {
    bytes: &'a [u8],
}
impl<'a> Modf<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.one(bytes.len())?;
        if bytes.len() != 64 {
            return Err(Error::Framing);
        }
        finite(&vector(bytes, 8)?)?;
        finite(&vector(bytes, 20)?)?;
        let low = vector(bytes, 32)?;
        let high = vector(bytes, 44)?;
        valid_bounds(&[low[0], low[1], low[2], high[0], high[1], high[2]])?;
        Ok(Self { bytes })
    }
    pub fn source_id(&self) -> u32 {
        word(self.bytes, 0).expect("validated MODF")
    }
    pub fn unique_id(&self) -> u32 {
        word(self.bytes, 4).expect("validated MODF")
    }
    pub fn position_bits(&self) -> [u32; 3] {
        vector(self.bytes, 8).expect("validated MODF")
    }
    pub fn rotation_bits(&self) -> [u32; 3] {
        vector(self.bytes, 20).expect("validated MODF")
    }
    pub fn bounds_bits(&self) -> [u32; 6] {
        let low = vector(self.bytes, 32).expect("validated MODF");
        let high = vector(self.bytes, 44).expect("validated MODF");
        [low[0], low[1], low[2], high[0], high[1], high[2]]
    }
    pub fn flags(&self) -> u16 {
        short(self.bytes, 56)
    }
    pub fn doodad_set(&self) -> u16 {
        short(self.bytes, 58)
    }
    pub fn name_set(&self) -> u16 {
        short(self.bytes, 60)
    }
    pub fn scale(&self) -> u16 {
        short(self.bytes, 62)
    }
    pub fn uses_file_data_id(&self) -> bool {
        self.flags() & 8 != 0
    }
}
