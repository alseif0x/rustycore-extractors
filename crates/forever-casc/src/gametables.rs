// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, independent admission of Forever GameTable files. The original
//! six-table API remains separate from the opt-in twenty-file pinned catalog.
//!
//! File identities are facts from TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
//! `src/tools/map_extractor/System.cpp:1346-1367`. Column counts and physical
//! row ordering follow `GameTables.h` and `GameTables.cpp:44-100` at that pin.
//! No upstream implementation is copied here. This module performs no I/O.
//!
//! The first text column is ignored, and row zero is an extra positive-zero
//! row. Text is never rewritten. Numeric admission accepts finite decimal
//! values through f64 then f32, matching the conservative RustyCore reader
//! policy. It does not claim universal equivalence to C++ `std::stold` then
//! f32, or to C++'s conversion-error fallback to zero. Independent all-cell
//! reader/oracle comparison remains an acceptance requirement.

pub mod batch;
pub mod catalog;
pub mod catalog_batch;
pub mod catalog_manifest;
pub mod contract;
pub mod manifest;
pub mod options;
mod output;
#[cfg(test)]
mod tests;
pub mod verify;

/// Admission limits, not limits of the source format.
pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_DATA_ROWS: usize = 100_000;
pub const MAX_VALUES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameTable {
    BaseMp,
    HpPerSta,
    Xp,
    SpellScaling,
    CombatRatingsMultByILvl,
    StaminaMultByILvl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Initial,
    SpellValues,
}

impl Group {
    pub const fn tables(self) -> [GameTable; 3] {
        match self {
            Self::Initial => [GameTable::BaseMp, GameTable::HpPerSta, GameTable::Xp],
            Self::SpellValues => [
                GameTable::SpellScaling,
                GameTable::CombatRatingsMultByILvl,
                GameTable::StaminaMultByILvl,
            ],
        }
    }
}

impl GameTable {
    pub const ALL: [Self; 6] = [
        Self::BaseMp,
        Self::HpPerSta,
        Self::Xp,
        Self::SpellScaling,
        Self::CombatRatingsMultByILvl,
        Self::StaminaMultByILvl,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BaseMp => "BaseMp",
            Self::HpPerSta => "HpPerSta",
            Self::Xp => "xp",
            Self::SpellScaling => "SpellScaling",
            Self::CombatRatingsMultByILvl => "CombatRatingsMultByILvl",
            Self::StaminaMultByILvl => "StaminaMultByILvl",
        }
    }

    /// Exact, case-sensitive source filename; never supplied by an input cell.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::BaseMp => "BaseMp.txt",
            Self::HpPerSta => "HpPerSta.txt",
            Self::Xp => "xp.txt",
            Self::SpellScaling => "SpellScaling.txt",
            Self::CombatRatingsMultByILvl => "CombatRatingsMultByILvl.txt",
            Self::StaminaMultByILvl => "StaminaMultByILvl.txt",
        }
    }

    pub const fn relative_path(self) -> &'static str {
        match self {
            Self::BaseMp => "gt/BaseMp.txt",
            Self::HpPerSta => "gt/HpPerSta.txt",
            Self::Xp => "gt/xp.txt",
            Self::SpellScaling => "gt/SpellScaling.txt",
            Self::CombatRatingsMultByILvl => "gt/CombatRatingsMultByILvl.txt",
            Self::StaminaMultByILvl => "gt/StaminaMultByILvl.txt",
        }
    }

    pub const fn file_id(self) -> u32 {
        match self {
            Self::BaseMp => 1_391_664,
            Self::HpPerSta => 1_391_642,
            Self::Xp => 1_391_661,
            Self::SpellScaling => 1_391_660,
            Self::CombatRatingsMultByILvl => 1_391_670,
            Self::StaminaMultByILvl => 1_980_632,
        }
    }

    /// Number of float cells, excluding the ignored first text column.
    pub const fn value_columns(self) -> usize {
        match self {
            Self::BaseMp => 15,
            Self::HpPerSta => 1,
            Self::Xp => 5,
            Self::SpellScaling => 24,
            Self::CombatRatingsMultByILvl | Self::StaminaMultByILvl => 4,
        }
    }

    pub const fn group(self) -> Group {
        match self {
            Self::BaseMp | Self::HpPerSta | Self::Xp => Group::Initial,
            Self::SpellScaling | Self::CombatRatingsMultByILvl | Self::StaminaMultByILvl => {
                Group::SpellValues
            }
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Utf8,
    Text,
    Header,
    Row,
    Numeric,
    Schema,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "gt_policy_limit",
            Self::Utf8 => "gt_invalid_utf8",
            Self::Text => "gt_invalid_text",
            Self::Header => "gt_header_columns",
            Self::Row => "gt_row_columns",
            Self::Numeric => "gt_numeric_admission",
            Self::Schema => "gt_schema_unknown",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

/// Safe counts for manifests and reports; no input text or numeric cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub bytes: usize,
    pub value_columns: usize,
    pub data_rows: usize,
    pub consumer_rows_including_zero: usize,
    /// A blank or one-token data line stopped source-style traversal.
    /// Remaining raw bytes are preserved by the caller, not parsed as rows.
    pub terminated_early: bool,
}

/// Admitted numeric bits in physical source order, including unused row zero.
/// Does not implement Debug: accidental diagnostics must not print cells.
pub struct ParsedTable {
    table: GameTable,
    facts: Facts,
    rows: Vec<Vec<u32>>,
}

impl ParsedTable {
    pub const fn table(&self) -> GameTable {
        self.table
    }

    pub const fn facts(&self) -> Facts {
        self.facts
    }

    /// Index zero is the implicit source row, not a parsed first-column ID.
    pub fn row(&self, physical_index: usize) -> Option<&[u32]> {
        self.rows.get(physical_index).map(Vec::as_slice)
    }

    /// Bitwise values for private all-cell comparison. Do not log this output.
    pub fn rows(&self) -> &[Vec<u32>] {
        &self.rows
    }
}

#[derive(Clone, Copy)]
struct Limits {
    bytes: usize,
    data_rows: usize,
    values: usize,
}

/// Parse bounded complete bytes without acquiring, rewriting or publishing them.
pub fn parse(bytes: &[u8], table: GameTable) -> Result<ParsedTable, Error> {
    parse_bounded(
        bytes,
        table,
        Limits {
            bytes: MAX_FILE_BYTES,
            data_rows: MAX_DATA_ROWS,
            values: MAX_VALUES,
        },
    )
}

/// Numeric bits for the pinned known-column subset. No Debug implementation:
/// private all-cell comparison must not accidentally expose numeric cells.
pub struct NumericTable {
    facts: Facts,
    rows: Vec<Vec<u32>>,
}

impl NumericTable {
    pub const fn facts(&self) -> Facts {
        self.facts
    }

    pub fn row(&self, physical_index: usize) -> Option<&[u32]> {
        self.rows.get(physical_index).map(Vec::as_slice)
    }

    pub fn rows(&self) -> &[Vec<u32>] {
        &self.rows
    }
}

fn parse_bounded(bytes: &[u8], table: GameTable, limits: Limits) -> Result<ParsedTable, Error> {
    let numeric = parse_columns(bytes, table.value_columns(), limits)?;
    Ok(ParsedTable {
        table,
        facts: numeric.facts,
        rows: numeric.rows,
    })
}

fn parse_columns(bytes: &[u8], columns: usize, limits: Limits) -> Result<NumericTable, Error> {
    if columns == 0 || columns > 32 {
        return Err(Error::Schema);
    }
    if bytes.len() > limits.bytes {
        return Err(Error::Limit);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Utf8)?;
    // NUL is outside this text admission policy, even after a logical stop.
    if bytes.contains(&0) {
        return Err(Error::Text);
    }
    let expected_tokens = columns.checked_add(1).ok_or(Error::Limit)?;
    let mut lines = text.split_terminator('\n');
    let header = lines.next().ok_or(Error::Header)?;
    if header.split('\t').filter(|token| !token.is_empty()).count() != expected_tokens {
        return Err(Error::Header);
    }
    if columns > limits.values {
        return Err(Error::Limit);
    }
    let mut rows = vec![vec![0; columns]];
    let mut terminated_early = false;
    for raw_line in lines {
        let line = raw_line.split('\r').next().ok_or(Error::Text)?;
        // Tokenization must retain internal empty cells, but trailing tabs do
        // not create extra fields in the source loader.
        let significant = line.trim_end_matches('\t');
        let token_count = significant.split('\t').count();
        if token_count <= 1 {
            terminated_early = true;
            break;
        }
        if token_count != expected_tokens {
            return Err(Error::Row);
        }
        let next_rows = rows.len().checked_add(1).ok_or(Error::Limit)?;
        if rows.len() > limits.data_rows
            || next_rows.checked_mul(columns).ok_or(Error::Limit)? > limits.values
        {
            return Err(Error::Limit);
        }
        let mut row = Vec::with_capacity(columns);
        for cell in significant.split('\t').skip(1) {
            row.push(decimal_bits(cell)?);
        }
        rows.push(row);
    }
    Ok(NumericTable {
        facts: Facts {
            bytes: bytes.len(),
            value_columns: columns,
            data_rows: rows.len() - 1,
            consumer_rows_including_zero: rows.len(),
            terminated_early,
        },
        rows,
    })
}

/// A decimal grammar avoids accidental acceptance of nonfinite spellings or
/// syntax extensions. Preserve signed zero, reject both overflow and underflow.
fn decimal_bits(cell: &str) -> Result<u32, Error> {
    let token = cell.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
    let bytes = token.as_bytes();
    let mut offset = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut mantissa_digits = 0usize;
    let mut nonzero = false;
    while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
        nonzero |= bytes[offset] != b'0';
        mantissa_digits += 1;
        offset += 1;
    }
    if bytes.get(offset) == Some(&b'.') {
        offset += 1;
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            nonzero |= bytes[offset] != b'0';
            mantissa_digits += 1;
            offset += 1;
        }
    }
    if mantissa_digits == 0 {
        return Err(Error::Numeric);
    }
    if matches!(bytes.get(offset), Some(b'e' | b'E')) {
        offset += 1;
        if matches!(bytes.get(offset), Some(b'+' | b'-')) {
            offset += 1;
        }
        let exponent_start = offset;
        while bytes.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if offset == exponent_start {
            return Err(Error::Numeric);
        }
    }
    if offset != bytes.len() {
        return Err(Error::Numeric);
    }
    let wide = token.parse::<f64>().map_err(|_| Error::Numeric)?;
    let narrow = wide as f32;
    if !wide.is_finite()
        || !narrow.is_finite()
        || (wide == 0.0 && nonzero)
        || (narrow == 0.0 && wide != 0.0)
    {
        return Err(Error::Numeric);
    }
    Ok(narrow.to_bits())
}
