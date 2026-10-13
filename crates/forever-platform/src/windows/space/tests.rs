// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: synthetic native same-handle capacity observation.
use super::*;
#[test]
fn caller_quotas_units_and_missing_inode_count_are_explicit() {
    let value = from_counts(
        42,
        ffi::Counts {
            total: 10,
            caller: 3,
            actual: 20,
            sectors: 8,
            bytes_per_sector: 512,
        },
    )
    .unwrap();
    assert_eq!(
        value,
        Available {
            volume: 42,
            allocation_unit: 4096,
            bytes: 12288,
            inodes: None
        }
    );
    assert_eq!(
        from_counts(
            0,
            ffi::Counts {
                total: 0,
                caller: 0,
                actual: 0,
                sectors: 1,
                bytes_per_sector: 512
            }
        )
        .unwrap()
        .bytes,
        0
    );
}
#[test]
fn anomalous_negative_units_and_overflow_fail_closed() {
    for (total, caller, actual, sectors, bytes) in [
        (-1, 0, 0, 1, 512),
        (1, -1, 1, 1, 512),
        (1, 0, -1, 1, 512),
        (1, 2, 3, 1, 512),
        (3, 2, 1, 1, 512),
        (3, 1, 3, 0, 512),
        (3, 1, 3, 1, 0),
    ] {
        assert_eq!(
            from_counts(
                0,
                ffi::Counts {
                    total,
                    caller,
                    actual,
                    sectors,
                    bytes_per_sector: bytes
                }
            ),
            Err(Error::Invalid)
        );
    }
    assert_eq!(
        from_counts(
            0,
            ffi::Counts {
                total: i64::MAX,
                caller: i64::MAX,
                actual: i64::MAX,
                sectors: u32::MAX,
                bytes_per_sector: u32::MAX
            }
        ),
        Err(Error::Overflow)
    );
}
#[test]
fn real_ntfs_observation_uses_directory_volume_and_survives_child_creation() {
    let path = std::env::temp_dir().join(format!(
        "forever-space-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    {
        let directory = ReadDirectory::open(&path, crate::path::Limits::default()).unwrap();
        let value = available(&directory).unwrap();
        assert_eq!(value.volume, directory.identity().volume);
        assert!(value.allocation_unit > 0);
        assert_eq!(value.inodes, None);
        std::fs::write(path.join("child"), b"synthetic").unwrap();
        assert_eq!(available(&directory).unwrap().volume, value.volume);
    }
    std::fs::remove_dir_all(path).unwrap();
}
