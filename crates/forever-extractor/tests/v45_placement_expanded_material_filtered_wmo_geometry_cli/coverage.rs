// SPDX-License-Identifier: GPL-3.0-or-later

use crate::v43_coverage::Coverage;

pub const PATH: &str =
    "vmaps/available-placement-expanded-material-filtered-wmo-geometry-coverage-v18.bin";
pub const COVERAGE_PROFILE: &str =
    "linux70450-placement-expanded-material-filtered-wmo-coverage-v18";
pub const SOURCE_PROFILE: &str = "linux70450-placement-expanded-material-filtered-wmo-v1";
pub const DOMAIN_PROFILE: &str = "placement-expanded-material-filtered-wmo-v1";
pub const RESOURCE_PROFILE: &str = "linux-collision-placement-expanded-resources-v1";

const V43_IDENTITIES: [(&str, &str); 4] = [
    (
        COVERAGE_PROFILE,
        "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16",
    ),
    (
        SOURCE_PROFILE,
        "linux70450-model-bytes-expanded-material-filtered-wmo-v1",
    ),
    (
        DOMAIN_PROFILE,
        "model-bytes-expanded-material-filtered-wmo-v1",
    ),
    (
        RESOURCE_PROFILE,
        "linux-collision-model-bytes-expanded-resources-v1",
    ),
];

pub const V43_COVERAGE_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16";
pub const V43_SOURCE_PROFILE: &str = "linux70450-model-bytes-expanded-material-filtered-wmo-v1";
pub const V43_DOMAIN_PROFILE: &str = "model-bytes-expanded-material-filtered-wmo-v1";
pub const V43_RESOURCE_PROFILE: &str = "linux-collision-model-bytes-expanded-resources-v1";

pub fn normalize_for_v43(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = bytes.to_vec();
    assert!(normalized.len() >= 12);
    assert_eq!(
        u32::from_le_bytes(normalized[8..12].try_into().unwrap()),
        18
    );
    normalized[8..12].copy_from_slice(&16_u32.to_le_bytes());
    let mut replacements = 0;
    for (v45, v43) in V43_IDENTITIES {
        let (updated, count) = replace_text_fields(normalized, v45.as_bytes(), v43.as_bytes());
        normalized = updated;
        replacements += count;
    }
    assert!(replacements > 0, "missing v45 coverage identities");
    normalized
}

pub fn denormalize_from_v43(mut bytes: Vec<u8>) -> Vec<u8> {
    assert!(bytes.len() >= 12);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 16);
    bytes[8..12].copy_from_slice(&18_u32.to_le_bytes());
    let mut replacements = 0;
    for (v45, v43) in V43_IDENTITIES {
        let (updated, count) = replace_text_fields(bytes, v43.as_bytes(), v45.as_bytes());
        bytes = updated;
        replacements += count;
    }
    assert!(replacements > 0, "missing v43 coverage identities");
    bytes
}

pub fn parse(bytes: &[u8]) -> Coverage {
    Coverage::parse(&normalize_for_v43(bytes)).unwrap()
}

fn replace_text_fields(mut bytes: Vec<u8>, from: &[u8], to: &[u8]) -> (Vec<u8>, usize) {
    assert!(!from.is_empty());
    let mut output = Vec::with_capacity(bytes.len());
    let mut at = 0;
    let mut replacements = 0;
    while at < bytes.len() {
        let text_start = at.checked_add(4);
        let text_end = text_start.and_then(|start| start.checked_add(from.len()));
        let is_field = text_start.is_some_and(|start| {
            start <= bytes.len()
                && u32::from_le_bytes(bytes[at..start].try_into().unwrap()) as usize == from.len()
        }) && text_end
            .is_some_and(|end| end <= bytes.len() && bytes[text_start.unwrap()..end] == *from);
        if is_field {
            output.extend_from_slice(&u32::try_from(to.len()).unwrap().to_le_bytes());
            output.extend_from_slice(to);
            at = text_end.unwrap();
            replacements += 1;
        } else {
            output.push(bytes[at]);
            at += 1;
        }
    }
    bytes.clear();
    (output, replacements)
}

pub fn replace_omission(
    bytes: &[u8],
    list_index: usize,
    ids: &[u32],
) -> Result<Vec<u8>, &'static str> {
    let normalized = normalize_for_v43(bytes);
    let parsed = Coverage::parse(&normalized)?;
    let replaced = parsed.replace_omission(&normalized, list_index, ids)?;
    Ok(denormalize_from_v43(replaced))
}

pub fn replace_omissions(bytes: &[u8], omissions: &[Vec<u32>; 5]) -> Result<Vec<u8>, &'static str> {
    let normalized = normalize_for_v43(bytes);
    let parsed = Coverage::parse(&normalized)?;
    let mut replaced = normalized;
    for index in (0..omissions.len()).rev() {
        replaced = parsed.replace_omission(&replaced, index, &omissions[index])?;
    }
    Ok(denormalize_from_v43(replaced))
}

#[cfg(test)]
mod tests {
    use super::{denormalize_from_v43, normalize_for_v43};

    #[test]
    fn v18_profile_identities_round_trip_to_v43_names() {
        let mut source = b"RCVGEO1\0".to_vec();
        source.extend_from_slice(&18_u32.to_le_bytes());
        source.extend_from_slice(&u32::try_from("context".len()).unwrap().to_le_bytes());
        source.extend_from_slice(b"context");
        for (v45, _) in super::V43_IDENTITIES {
            source.extend_from_slice(&u32::try_from(v45.len()).unwrap().to_le_bytes());
            source.extend_from_slice(v45.as_bytes());
        }
        let normalized = normalize_for_v43(&source);
        assert_eq!(
            u32::from_le_bytes(normalized[8..12].try_into().unwrap()),
            16
        );
        for value in [
            super::V43_COVERAGE_PROFILE,
            super::V43_SOURCE_PROFILE,
            super::V43_DOMAIN_PROFILE,
            super::V43_RESOURCE_PROFILE,
        ] {
            assert!(
                normalized
                    .windows(value.len())
                    .any(|window| window == value.as_bytes())
            );
        }
        assert_eq!(denormalize_from_v43(normalized), source);
    }

    #[test]
    fn normalization_rejects_wrong_version_and_missing_identity_fields() {
        assert!(std::panic::catch_unwind(|| normalize_for_v43(b"short")).is_err());
        assert!(std::panic::catch_unwind(|| denormalize_from_v43(b"short".to_vec())).is_err());
        let mut header = b"RCVGEO1\0".to_vec();
        header.extend_from_slice(&18_u32.to_le_bytes());
        assert!(std::panic::catch_unwind(|| normalize_for_v43(&header)).is_err());
    }
}
