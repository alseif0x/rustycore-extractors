// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: host-independent conservative Windows path admission.

use crate::Error;

pub const MAX_ANCESTORS: usize = 63;
pub const MAX_PATH_UNITS: usize = 32_760;
pub const MAX_READ_BYTES: usize = 64 * 1024 * 1024;

/// Explicit downward-only limits. A read lease holds at most ancestors + one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_ancestors: usize,
    pub max_path_units: usize,
    pub max_read_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ancestors: MAX_ANCESTORS,
            max_path_units: MAX_PATH_UNITS,
            max_read_bytes: MAX_READ_BYTES,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<Self, Error> {
        if self.max_ancestors == 0
            || self.max_ancestors > MAX_ANCESTORS
            || self.max_path_units < 4
            || self.max_path_units > MAX_PATH_UNITS
            || self.max_read_bytes == 0
            || self.max_read_bytes > MAX_READ_BYTES
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
}

/// Validated absolute drive path. No Debug implementation exposes its contents.
/// Parsing is independent of the host OS; the final component denotes a file.
pub struct AbsolutePath {
    text: String,
    ends: Vec<usize>,
}
impl AbsolutePath {
    pub fn parse(text: &str, limits: Limits) -> Result<Self, Error> {
        let limits = limits.validate()?;
        let b = text.as_bytes();
        if b.len() < 4 || !b[0].is_ascii_alphabetic() || b[1..3] != *b":\\" {
            return Err(Error::Path);
        }
        if text.len() > limits.max_path_units * 3
            || text.encode_utf16().count() > limits.max_path_units
        {
            return Err(Error::Limit);
        }
        // One root handle plus every non-final component: component count equals
        // ancestor count. Reject before allocating any per-component storage.
        let count = text[3..].split('\\').count();
        if count > limits.max_ancestors {
            return Err(Error::Limit);
        }
        let mut ends = Vec::new();
        ends.try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        let mut offset = 3;
        for component in text[3..].split('\\') {
            validate_component(component)?;
            offset += component.len();
            ends.push(offset);
            offset += 1;
        }
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| Error::Allocation)?;
        owned.push_str(text);
        Ok(Self { text: owned, ends })
    }
    pub fn as_str(&self) -> &str {
        &self.text
    }
    pub fn ancestor_count(&self) -> usize {
        self.ends.len()
    }
    /// Root followed by each non-final prefix, then the full file path.
    pub fn prefixes(&self) -> impl ExactSizeIterator<Item = &str> {
        (0..self.ends.len() + 1).map(|i| {
            let end = if i == 0 { 3 } else { self.ends[i - 1] };
            &self.text[..end]
        })
    }
}

fn validate_component(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with(['.', ' '])
        || value
            .chars()
            .any(|c| c <= '\u{1f}' || c == '\u{7f}' || "<>:\"/|?*".contains(c))
        || value.encode_utf16().count() > 255
    {
        return Err(Error::Path);
    }
    // Win32 reserves the stem even with an extension. Spaces before a period
    // are rejected for reserved stems too (e.g. CON .txt).
    let stem = value.split('.').next().unwrap_or("").trim_end_matches(' ');
    let reserved = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
    if reserved.iter().any(|name| stem.eq_ignore_ascii_case(name)) {
        return Err(Error::Path);
    }
    if stem.len() >= 4 {
        let prefix = &stem.as_bytes()[..3];
        if (prefix.eq_ignore_ascii_case(b"COM") || prefix.eq_ignore_ascii_case(b"LPT"))
            && matches!(
                &stem[3..],
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        {
            return Err(Error::Path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_unicode_and_exact_prefixes() {
        let p = AbsolutePath::parse("c:\\cliente español\\Data\\archivo.db2", Limits::default())
            .unwrap();
        assert_eq!(p.ancestor_count(), 3);
        assert_eq!(
            p.prefixes().collect::<Vec<_>>(),
            [
                "c:\\",
                "c:\\cliente español",
                "c:\\cliente español\\Data",
                "c:\\cliente español\\Data\\archivo.db2"
            ]
        );
        assert!(AbsolutePath::parse("Z:\\x", Limits::default()).is_ok());
        assert!(AbsolutePath::parse("C:\\COM0\\LPT10\\console", Limits::default()).is_ok());
    }
    #[test]
    fn reject_device_unc_relative_aliases() {
        for p in [
            "",
            "C:",
            "C:\\",
            "C:x",
            "\\\\server\\share\\f",
            "\\\\?\\C:\\f",
            "\\\\.\\C:\\f",
            "/tmp/file",
            "C:/file",
            "C:\\a/f",
            "C:\\..\\f",
            "C:\\.\\f",
            "C:\\a\\\\f",
            "C:\\f\\",
            "C:\\f:stream",
            "C:\\f.",
            "C:\\f ",
            "C:\\a?",
            "C:\\a*",
            "C:\\a\0",
            "C:\\a\u{7f}",
        ] {
            assert!(
                matches!(AbsolutePath::parse(p, Limits::default()), Err(Error::Path)),
                "case {p:?}"
            );
        }
    }
    #[test]
    fn reject_reserved_names_case_extensions_and_superscripts() {
        for name in [
            "CON",
            "con.txt",
            "CON .txt",
            "PRN",
            "AUX",
            "nul",
            "CONIN$",
            "conout$.txt",
            "COM1",
            "com9.txt",
            "LPT1",
            "LPT9.ext",
            "COM¹",
            "COM².foo",
            "COM³",
            "LPT¹",
            "LPT²",
            "LPT³",
        ] {
            assert!(
                matches!(
                    AbsolutePath::parse(&format!("C:\\{name}\\file"), Limits::default()),
                    Err(Error::Path)
                ),
                "case {name}"
            );
        }
    }
    #[test]
    fn exact_depth_and_component_limits() {
        let p = format!("C:\\{}", vec!["x"; 63].join("\\"));
        assert_eq!(
            AbsolutePath::parse(&p, Limits::default())
                .unwrap()
                .ancestor_count(),
            63
        );
        assert!(matches!(
            AbsolutePath::parse(&format!("{p}\\x"), Limits::default()),
            Err(Error::Limit)
        ));
        assert!(
            AbsolutePath::parse(&format!("C:\\{}", "a".repeat(255)), Limits::default()).is_ok()
        );
        assert!(matches!(
            AbsolutePath::parse(&format!("C:\\{}", "a".repeat(256)), Limits::default()),
            Err(Error::Path)
        ));
    }
    #[test]
    fn downward_limits_and_utf16_accounting() {
        assert!(
            Limits {
                max_ancestors: 65,
                ..Limits::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            Limits {
                max_read_bytes: MAX_READ_BYTES + 1,
                ..Limits::default()
            }
            .validate()
            .is_err()
        );
        let limits = Limits {
            max_path_units: 5,
            ..Limits::default()
        };
        assert!(AbsolutePath::parse("C:\\🦀", limits).is_ok());
        assert!(matches!(
            AbsolutePath::parse("C:\\🦀x", limits),
            Err(Error::Limit)
        ));
        assert!(matches!(
            AbsolutePath::parse(
                "C:\\a\\b",
                Limits {
                    max_ancestors: 1,
                    ..Limits::default()
                }
            ),
            Err(Error::Limit)
        ));
    }
}
