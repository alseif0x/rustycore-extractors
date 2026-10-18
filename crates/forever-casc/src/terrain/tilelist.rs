// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original MAPS v10 whole-map tile-list codec.
//! System.cpp:1135-1144 at 02245dcd: std::bitset::to_string emits bit 4095
//! first. Coordinates remain y * 64 + x; this is not row-forward ASCII.
//! A syntactically valid list alone does not prove the referenced files exist.
pub const BYTES: usize = 12 + 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Build,
    Coordinates,
    Encoding,
}
pub struct TileList {
    bytes: [u8; BYTES],
}
impl TileList {
    pub fn new(build: u32) -> Result<Self, Error> {
        if !matches!(build, 70170 | 70205) {
            return Err(Error::Build);
        }
        Ok(Self::header(build))
    }
    /// Explicit70374 codec; old new/verify/present retain their older gates.
    pub fn new_70374(build: u32) -> Result<Self, Error> {
        if build != 70374 {
            return Err(Error::Build);
        }
        Ok(Self::header(build))
    }
    /// Explicit70450 codec; old new/verify/present retain their older gates.
    pub fn new_70450(build: u32) -> Result<Self, Error> {
        if build != 70450 {
            return Err(Error::Build);
        }
        Ok(Self::header(build))
    }
    fn header(build: u32) -> Self {
        let mut bytes = [b'0'; BYTES];
        bytes[..4].copy_from_slice(b"MAPS");
        bytes[4..8].copy_from_slice(&10u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&build.to_le_bytes());
        Self { bytes }
    }
    pub fn set(&mut self, x: u8, y: u8) -> Result<(), Error> {
        self.bytes[index(x, y)?] = b'1';
        Ok(())
    }
    pub const fn bytes(&self) -> &[u8; BYTES] {
        &self.bytes
    }
}
pub fn verify(bytes: &[u8], build: u32) -> Result<(), Error> {
    let header = TileList::new(build)?;
    verify_header(bytes, &header)
}
/// Explicit70374 list verification; no source or referenced-file authentication.
pub fn verify_70374(bytes: &[u8], build: u32) -> Result<(), Error> {
    verify_header(bytes, &TileList::new_70374(build)?)
}
/// Explicit70450 list verification; no source or referenced-file authentication.
pub fn verify_70450(bytes: &[u8], build: u32) -> Result<(), Error> {
    verify_header(bytes, &TileList::new_70450(build)?)
}
fn verify_header(bytes: &[u8], header: &TileList) -> Result<(), Error> {
    if bytes.len() != BYTES
        || bytes[..12] != header.bytes[..12]
        || bytes[12..].iter().any(|b| !matches!(b, b'0' | b'1'))
    {
        return Err(Error::Encoding);
    }
    Ok(())
}
pub fn present(bytes: &[u8], build: u32, x: u8, y: u8) -> Result<bool, Error> {
    verify(bytes, build)?;
    Ok(bytes[index(x, y)?] == b'1')
}
pub fn present_70374(bytes: &[u8], build: u32, x: u8, y: u8) -> Result<bool, Error> {
    verify_70374(bytes, build)?;
    Ok(bytes[index(x, y)?] == b'1')
}
pub fn present_70450(bytes: &[u8], build: u32, x: u8, y: u8) -> Result<bool, Error> {
    verify_70450(bytes, build)?;
    Ok(bytes[index(x, y)?] == b'1')
}
fn index(x: u8, y: u8) -> Result<usize, Error> {
    if x >= 64 || y >= 64 {
        return Err(Error::Coordinates);
    }
    Ok(12 + 4095 - (usize::from(y) * 64 + usize::from(x)))
}

#[cfg(test)]
mod tests;
