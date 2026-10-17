// SPDX-License-Identifier: GPL-3.0-or-later
use crate::Error;
use std::path::Path;
#[cfg(not(windows))]
use std::{fs::File, io::Read};
pub const PRODUCT: &str = "wow_classic_beta";
pub const SUPPORTED_CLIENTS: &[(&str, u32)] = &[
    ("1.60.1.70170", 70170),
    ("1.60.1.70205", 70205),
    ("1.60.1.70338", 70338),
];
const MAX_BUILD_INFO: u64 = 256 * 1024;

pub fn admit(client: &Path) -> Result<u32, Error> {
    admit_selected(client, parse)
}

/// Explicit Linux product-v10/v11 metadata admission; generic/native admission stays legacy.
#[cfg(target_os = "linux")]
pub fn admit_db2_70374(client: &Path) -> Result<u32, Error> {
    admit_selected(client, |info| parse_supported(info, DB2_70374_CLIENTS))
}

#[cfg(target_os = "linux")]
pub(crate) const DB2_70374_CLIENTS: &[(&str, u32)] = &[("1.60.1.70374", 70374)];

/// Separate opt-in Linux build70425 metadata admission; no component acceptance.
#[cfg(target_os = "linux")]
pub fn admit_db2_70425(client: &Path) -> Result<u32, Error> {
    admit_selected(client, |info| parse_supported(info, DB2_70425_CLIENTS))
}

#[cfg(target_os = "linux")]
pub(crate) const DB2_70425_CLIENTS: &[(&str, u32)] = &[("1.60.1.70425", 70425)];

/// Separate opt-in Linux build70450 metadata admission; no component acceptance.
#[cfg(target_os = "linux")]
pub fn admit_db2_70450(client: &Path) -> Result<u32, Error> {
    admit_selected(client, |info| parse_supported(info, DB2_70450_CLIENTS))
}

#[cfg(target_os = "linux")]
pub(crate) const DB2_70450_CLIENTS: &[(&str, u32)] = &[("1.60.1.70450", 70450)];

fn admit_selected(client: &Path, select: fn(&str) -> Result<u32, Error>) -> Result<u32, Error> {
    #[cfg(windows)]
    let bytes = {
        use forever_platform::{
            path::Limits,
            windows::{LinkPolicy, ReadFile},
        };
        let map = |error| match error {
            forever_platform::Error::Limit | forever_platform::Error::Allocation => Error::Limit,
            forever_platform::Error::Changed => {
                Error::LocalStore(crate::local_store::Error::InputChanged)
            }
            _ => Error::Input,
        };
        let file = ReadFile::open(
            &client.join(".build.info"),
            Limits::default(),
            LinkPolicy::Allow,
        )
        .map_err(map)?;
        file.read_all(MAX_BUILD_INFO as usize).map_err(map)?
    };
    #[cfg(not(windows))]
    let bytes = {
        let file = File::open(client.join(".build.info")).map_err(|_| Error::Input)?;
        let mut bytes = Vec::new();
        file.take(MAX_BUILD_INFO + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Input)?;
        if bytes.len() as u64 > MAX_BUILD_INFO {
            return Err(Error::Limit);
        }
        bytes
    };
    let info = std::str::from_utf8(&bytes).map_err(|_| Error::Metadata)?;
    let build = select(info)?;
    native_shape(info)?;
    Ok(build)
}

// CascFiles.cpp::ParseFile_BuildInfo indexes exact typed columns and calls
// strcmp on Active without a null check. Reject malformed metadata BEFORE FFI.
fn native_shape(info: &str) -> Result<(), Error> {
    if info.contains('\0') {
        return Err(Error::Metadata);
    }
    let mut lines = info.lines();
    let fields: Vec<_> = lines.next().ok_or(Error::Metadata)?.split('|').collect();
    if fields.len() > 64 {
        return Err(Error::Metadata);
    }
    let required = [
        "Product!STRING:0",
        "Active!DEC:1",
        "Version!STRING:0",
        "Build Key!HEX:16",
        "CDN Key!HEX:16",
        "Tags!STRING:0",
    ];
    let mut indexes = Vec::new();
    for field in required {
        indexes.push(
            fields
                .iter()
                .position(|value| *value == field)
                .ok_or(Error::Metadata)?,
        );
    }
    for line in lines.filter(|line| !line.is_empty()) {
        let row: Vec<_> = line.split('|').collect();
        if row.len() != fields.len()
            || !matches!(row[indexes[1]], "0" | "1")
            || row[indexes[0]].is_empty()
        {
            return Err(Error::Metadata);
        }
        if row[indexes[0]] == PRODUCT && row[indexes[1]] == "1" {
            for index in &indexes[3..5] {
                let key = row[*index].as_bytes();
                if key.len() != 32 || !key.iter().all(u8::is_ascii_hexdigit) {
                    return Err(Error::Metadata);
                }
            }
            if row[indexes[5]].len() < 4 || !row[indexes[5]].is_ascii() {
                return Err(Error::Metadata);
            }
        }
    }
    Ok(())
}

fn parse(info: &str) -> Result<u32, Error> {
    parse_supported(info, SUPPORTED_CLIENTS)
}

fn parse_supported(info: &str, supported: &[(&str, u32)]) -> Result<u32, Error> {
    let mut lines = info.lines();
    let fields: Vec<_> = lines
        .next()
        .ok_or(Error::Metadata)?
        .split('|')
        .map(|field| field.split('!').next().unwrap())
        .collect();
    let index = |name| {
        let mut matches = fields
            .iter()
            .enumerate()
            .filter(|(_, field)| **field == name);
        let (index, _) = matches.next().ok_or(Error::Metadata)?;
        if matches.next().is_some() {
            return Err(Error::Metadata);
        }
        Ok(index)
    };
    let product = index("Product")?;
    let active = index("Active")?;
    let version = index("Version")?;
    let mut selected = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let row: Vec<_> = line.split('|').collect();
        if row.len() != fields.len() {
            return Err(Error::Metadata);
        }
        if row[product] == PRODUCT && row[active] == "1" && selected.replace(row[version]).is_some()
        {
            return Err(Error::AmbiguousClient);
        }
    }
    supported
        .iter()
        .find(|(version, _)| Some(*version) == selected)
        .map(|(_, build)| *build)
        .ok_or(Error::UnsupportedClient)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_exact_active_product_not_first_row() {
        assert_eq!(
            parse(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0\n\
            wow_classic_era|1|1.15.9.70003\nwow_classic_beta|1|1.60.1.70170\n"
            ),
            Ok(70170)
        );
    }
    #[test]
    fn admits_each_exact_version_with_its_own_build() {
        for (version, build) in SUPPORTED_CLIENTS {
            assert_eq!(
                parse(&format!(
                    "Product|Active|Version\nwow_classic_beta|1|{version}\n"
                )),
                Ok(*build)
            );
        }
        assert_eq!(
            parse("Product|Active|Version\nwow_classic_beta|1|1.60.1.70206\n"),
            Err(Error::UnsupportedClient)
        );
        assert_eq!(
            parse(
                "Product|Active|Version\nwow_classic_beta|1|1.60.1.70170\nwow_classic_beta|1|1.60.1.70205\n"
            ),
            Err(Error::AmbiguousClient)
        );
    }
    #[test]
    fn wrong_product_version_or_inactive_rejects() {
        for row in [
            "wow|1|1.60.1.70170",
            "wow_classic_beta|1|1.60.1.70171",
            "wow_classic_beta|0|1.60.1.70170",
        ] {
            assert_eq!(
                parse(&format!("Product|Active|Version\n{row}\n")),
                Err(Error::UnsupportedClient)
            );
        }
    }
    #[test]
    fn duplicate_selection_and_malformed_columns_reject() {
        assert_eq!(
            parse(
                "Product|Active|Version\nwow_classic_beta|1|1.60.1.70170\n\
            wow_classic_beta|1|1.60.1.70170\n"
            ),
            Err(Error::AmbiguousClient)
        );
        for info in [
            "",
            "Product|Active\n",
            "Product|Active|Version|Version\n",
            "Product|Active|Version\nmalformed\n",
        ] {
            assert_eq!(parse(info), Err(Error::Metadata));
        }
    }
    #[test]
    fn native_requires_typed_columns_values_and_nonnull_active() {
        let good = "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\n\
            wow_classic_beta|1|1.60.1.70170|00000000000000000000000000000000|11111111111111111111111111111111|esES\n";
        assert_eq!(native_shape(good), Ok(()));
        for bad in [
            good.replace("Active!DEC:1", "Active"),
            good.replace("|1|", "||"),
            good.replace("|esES", "|"),
            good.replace("11111111111111111111111111111111", "bad"),
            good.replace("|esES", "|es\0ES"),
        ] {
            assert_eq!(native_shape(&bad), Err(Error::Metadata));
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
#[path = "admission/bootstrap_70425_tests.rs"]
mod bootstrap_70425_tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "admission/bootstrap_70450_tests.rs"]
mod bootstrap_70450_tests;
