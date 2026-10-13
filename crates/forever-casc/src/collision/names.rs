// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded ASCII routing/output-name preparation.
// Pin: adtfile.cpp:29-60; gameobject_extract.cpp:33-47; vmapexport.cpp:223-233.
use super::{Class, Error, InputRef};

pub(super) struct Prepared {
    pub id: Option<u32>,
    source: [u8; 1024],
    canonical: [u8; 1024],
    name: [u8; 1024],
    source_len: usize,
    name_len: usize,
}
impl Prepared {
    pub fn source(&self) -> &[u8] {
        &self.source[..self.source_len]
    }
    pub fn canonical(&self) -> &[u8] {
        &self.canonical[..self.source_len]
    }
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }
    pub fn new(input: InputRef<'_>, class: Class) -> Result<Self, Error> {
        let mut result = Self {
            id: None,
            source: [0; 1024],
            canonical: [0; 1024],
            name: [0; 1024],
            source_len: 0,
            name_len: 0,
        };
        match input {
            InputRef::FileDataId(id) => {
                if id == 0 || id == u32::MAX {
                    return Err(Error::Name);
                }
                result.id = Some(id);
                result.name[..4].copy_from_slice(b"FILE");
                for i in 0..8 {
                    result.name[4 + i] = b"0123456789ABCDEF"[((id >> (28 - i * 4)) & 15) as usize];
                }
                result.name[12..16].copy_from_slice(b".xxx");
                result.name_len = 16;
            }
            InputRef::LogicalName(name) => {
                if name.is_empty()
                    || name.len() > 1024
                    || name.iter().any(|b| *b == 0 || !b.is_ascii())
                {
                    return Err(Error::Name);
                }
                result.source[..name.len()].copy_from_slice(name);
                result.source_len = name.len();
                if class == Class::M2 {
                    if name.len() < 4 {
                        return Err(Error::Name);
                    }
                    let suffix = &name[name.len() - 4..];
                    if suffix.eq_ignore_ascii_case(b".mdx") || suffix.eq_ignore_ascii_case(b".mdl")
                    {
                        // gameobject_extract.cpp:40 replaces only the final
                        // two bytes with '2'; the original M/m is retained.
                        result.source[name.len() - 2] = b'2';
                        result.source_len -= 1;
                    }
                }
                for i in 0..result.source_len {
                    result.canonical[i] = if result.source[i] == b'/' {
                        b'\\'
                    } else {
                        result.source[i].to_ascii_uppercase()
                    };
                }
                let at = result
                    .source()
                    .iter()
                    .rposition(|b| matches!(b, b'/' | b'\\'))
                    .map_or(0, |i| i + 1);
                result.name_len = result.source_len - at;
                if result.name_len == 0 {
                    return Err(Error::Name);
                }
                result.name[..result.name_len]
                    .copy_from_slice(&result.source[at..result.source_len]);
                normalize(&mut result.name[..result.name_len]);
            }
        }
        if class == Class::Wmo
            && let Some(at) = result.name().iter().rposition(|b| *b == b'_')
            && result
                .name()
                .get(at + 1..at + 4)
                .is_some_and(|v| v.iter().all(u8::is_ascii_digit))
        {
            return Err(Error::GroupName);
        }
        Ok(result)
    }
}

fn normalize(name: &mut [u8]) {
    if name.starts_with(b"FILE") {
        return;
    }
    let mut i = name.len() - 1;
    while i > 0 && name[i] != b'.' {
        name[i] = name[i].to_ascii_lowercase();
        i -= 1;
    }
    while i > 0 {
        let previous_alpha = name[i - 1].is_ascii_alphabetic();
        if name[i].is_ascii_uppercase() && previous_alpha {
            name[i] = name[i].to_ascii_lowercase();
        } else if name[i].is_ascii_lowercase() && !previous_alpha {
            name[i] = name[i].to_ascii_uppercase();
        } else if name[i] == b' ' {
            name[i] = b'_';
        }
        i -= 1;
    }
}

#[cfg(test)]
fn normalize_basename(source: &[u8]) -> Result<Vec<u8>, Error> {
    if source.is_empty() || source.len() > 1024 || source.iter().any(|b| *b == 0 || !b.is_ascii()) {
        return Err(Error::Name);
    }
    let at = source
        .iter()
        .rposition(|b| matches!(b, b'/' | b'\\'))
        .map_or(0, |i| i + 1);
    let mut name = source[at..].to_vec();
    if name.is_empty() {
        return Err(Error::Name);
    }
    normalize(&mut name);
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_all_original_cpp_normalization_vectors() {
        fn unhex(s: &str) -> Vec<u8> {
            s.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut count = 0;
        for line in include_str!("../../../../tests/forever/fixtures/model-name-cpp-vectors.tsv")
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
        {
            let (source, expected) = line.split_once('\t').unwrap();
            assert_eq!(
                normalize_basename(&unhex(source)).unwrap(),
                unhex(expected),
                "vector {count}"
            );
            count += 1;
        }
        assert_eq!(count, 796);
    }
    #[test]
    fn rewrite_routing_is_separate_from_basename_and_fdid() {
        let p = Prepared::new(InputRef::LogicalName(b"A/BIG MODEL.MDX"), Class::M2).unwrap();
        assert_eq!(p.source(), b"A/BIG MODEL.M2");
        assert_eq!(p.name(), b"Big_Model.m2");
        assert_eq!(p.canonical(), b"A\\BIG MODEL.M2");
        let p = Prepared::new(InputRef::FileDataId(0xAB), Class::M2).unwrap();
        assert_eq!(p.name(), b"FILE000000AB.xxx");
        assert_eq!(
            Prepared::new(InputRef::LogicalName(b"x_123.wmo"), Class::Wmo).err(),
            Some(Error::GroupName)
        );
    }
    #[test]
    fn legacy_extension_preserves_m_case_before_file_prefix_normalization() {
        let cases: &[(&[u8], &[u8], &[u8])] = &[
            (b"dir/FILEfoo.MDX", b"dir/FILEfoo.M2", b"FILEfoo.M2"),
            (b"dir/FILEfoo.MdL", b"dir/FILEfoo.M2", b"FILEfoo.M2"),
            (b"dir/FILEfoo.mDX", b"dir/FILEfoo.m2", b"FILEfoo.m2"),
            (b"dir/FILEfoo.mdl", b"dir/FILEfoo.m2", b"FILEfoo.m2"),
            (b"dir/foo.MDX", b"dir/foo.M2", b"foo.m2"),
            (b"dir/foo.MdL", b"dir/foo.M2", b"foo.m2"),
            (b"dir/FILEfoo.M2", b"dir/FILEfoo.M2", b"FILEfoo.M2"),
        ];
        for &(source, rewritten, basename) in cases {
            let prepared = Prepared::new(InputRef::LogicalName(source), Class::M2).unwrap();
            assert_eq!(prepared.source(), rewritten);
            assert_eq!(prepared.name(), basename);
        }
        let wmo = Prepared::new(InputRef::LogicalName(b"dir/FILEfoo.MDX"), Class::Wmo).unwrap();
        assert_eq!(wmo.source(), b"dir/FILEfoo.MDX");
        assert_eq!(wmo.name(), b"FILEfoo.MDX");
    }
    #[test]
    fn ascii_limits_empty_and_file_prefix_are_explicit() {
        for name in [b"".as_slice(), b"x/", b"a\0b", &[0xff], b"abc"] {
            assert_eq!(
                Prepared::new(InputRef::LogicalName(name), Class::M2).err(),
                Some(Error::Name)
            );
        }
        let p = Prepared::new(InputRef::LogicalName(b"dir/FILEfoo.WMO"), Class::Wmo).unwrap();
        assert_eq!(p.name(), b"FILEfoo.WMO");
        let p = Prepared::new(InputRef::LogicalName(b"DIR/ABCDEF"), Class::Wmo).unwrap();
        assert_eq!(p.name(), b"Abcdef");
        assert_eq!(
            Prepared::new(InputRef::LogicalName(&[b'A'; 1025]), Class::Wmo).err(),
            Some(Error::Name)
        );
    }
}
