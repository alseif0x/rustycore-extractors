// SPDX-License-Identifier: GPL-3.0-or-later
use crate::Error;
use std::{fs::File, io::Read, path::Path};
pub const PRODUCT: &str = "wow_classic_beta";
pub const VERSION: &str = "1.60.1.70170";
pub const BUILD: u32 = 70170;
const MAX_BUILD_INFO: u64 = 256 * 1024;

pub fn admit(client: &Path) -> Result<(), Error> {
    let file = File::open(client.join(".build.info")).map_err(|_| Error::Input)?;
    let mut bytes = Vec::new();
    file.take(MAX_BUILD_INFO + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Input)?;
    if bytes.len() as u64 > MAX_BUILD_INFO {
        return Err(Error::Limit);
    }
    parse(std::str::from_utf8(&bytes).map_err(|_| Error::Metadata)?)
}

fn parse(info: &str) -> Result<(), Error> {
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
    if selected != Some(VERSION) {
        return Err(Error::UnsupportedClient);
    }
    Ok(())
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
            Ok(())
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
}
