// SPDX-License-Identifier: GPL-3.0-or-later

pub const PATH: &str =
    "vmaps/available-model-bytes-expanded-material-filtered-wmo-geometry-coverage-v16.bin";
pub const COVERAGE_PROFILE: &str =
    "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16";
pub const SOURCE_PROFILE: &str = "linux70450-model-bytes-expanded-material-filtered-wmo-v1";
pub const DOMAIN_PROFILE: &str = "model-bytes-expanded-material-filtered-wmo-v1";
pub const RESOURCE_PROFILE: &str = "linux-collision-model-bytes-expanded-resources-v1";
pub const LIST_ORDER: [&str; 5] = [
    "missing_m2",
    "missing_wmo",
    "empty_wmo",
    "unselected_wmo",
    "material_filtered_wmo",
];

#[derive(Debug, PartialEq, Eq)]
pub struct Coverage {
    pub version: u32,
    pub coverage_profile: String,
    pub source_profile: String,
    pub domain_profile: String,
    pub resource_profile: String,
    pub omissions: [Vec<u32>; 5],
    omission_ranges: [(usize, usize); 5],
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], &'static str> {
        let end = self.at.checked_add(count).ok_or("coverage bounds")?;
        let result = self.bytes.get(self.at..end).ok_or("coverage bounds")?;
        self.at = end;
        Ok(result)
    }

    fn u32(&mut self) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().map_err(|_| "coverage word")?,
        ))
    }

    fn u64(&mut self) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().map_err(|_| "coverage word")?,
        ))
    }

    fn bit(&mut self) -> Result<bool, &'static str> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("coverage bit"),
        }
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let length = usize::try_from(self.u32()?).map_err(|_| "coverage text length")?;
        if length > 4096 {
            return Err("coverage text limit");
        }
        std::str::from_utf8(self.take(length)?).map_err(|_| "coverage text")
    }

    fn list(&mut self) -> Result<Vec<u32>, &'static str> {
        let count = usize::try_from(self.u32()?).map_err(|_| "coverage count")?;
        if count > 4096 {
            return Err("coverage count");
        }
        let mut values = Vec::with_capacity(count);
        let mut previous = None;
        for _ in 0..count {
            let value = self.u32()?;
            if value == 0 || previous.is_some_and(|prior| prior >= value) {
                return Err("coverage list canonicality");
            }
            values.push(value);
            previous = Some(value);
        }
        Ok(values)
    }
}

impl Coverage {
    pub fn replace_omission(
        &self,
        bytes: &[u8],
        list_index: usize,
        ids: &[u32],
    ) -> Result<Vec<u8>, &'static str> {
        if list_index >= self.omissions.len() || !canonical_ordered_ids(ids) || ids.len() > 4097 {
            return Err("coverage list boundary");
        }
        let (start, end) = self.omission_ranges[list_index];
        let added_bytes = ids.len().checked_mul(4).ok_or("coverage bounds")?;
        let capacity = bytes
            .len()
            .checked_sub(end - start)
            .and_then(|length| length.checked_add(4 + added_bytes))
            .ok_or("coverage bounds")?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(capacity)
            .map_err(|_| "coverage allocation")?;
        output.extend_from_slice(&bytes[..start]);
        output.extend_from_slice(
            &u32::try_from(ids.len())
                .map_err(|_| "coverage count")?
                .to_le_bytes(),
        );
        for id in ids {
            output.extend_from_slice(&id.to_le_bytes());
        }
        output.extend_from_slice(&bytes[end..]);
        Ok(output)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 128 * 1024 * 1024 {
            return Err("coverage size");
        }
        let mut reader = Reader { bytes, at: 0 };
        if reader.take(8)? != b"RCVGEO1\0" || reader.u32()? != 16 {
            return Err("coverage header or version");
        }
        let context = reader.text()?;
        if !context.contains("70450") || !context.contains("17") {
            return Err("coverage context scope");
        }
        let coverage_profile = reader.text()?.to_owned();
        let source_profile = reader.text()?.to_owned();
        let domain_profile = reader.text()?.to_owned();
        if coverage_profile != COVERAGE_PROFILE
            || source_profile != SOURCE_PROFILE
            || domain_profile != DOMAIN_PROFILE
        {
            return Err("coverage profile");
        }

        if !reader.bit()? {
            return Err("coverage missing-WDT policy");
        }
        for expected in [
            "excluded-by-profile-v1",
            "placement-only-v1",
            "available-section-map-partial-scope-v1",
        ] {
            if reader.text()? != expected {
                return Err("coverage policy");
            }
        }
        if reader.bit()?
            || reader.text()? != "excluded"
            || reader.text()? != "excluded_by_profile"
            || reader.bit()?
            || reader.text()? != "placement_only"
            || reader.text()? != "complete-LiquidMaterial-LiquidType-v1"
            || reader.text()? != "out_of_scope"
            || reader.bit()?
            || reader.bit()?
        {
            return Err("coverage exclusion or completion policy");
        }
        let _gameobject_file_data_id = reader.u32()?;
        let _gameobject_startup_reads = reader.u64()?;
        if reader.u32()? != 0x3ff {
            return Err("coverage policy facts");
        }

        let _map_file_data_id = reader.u32()?;
        let _map_declared_bytes = reader.u64()?;
        let _map_available_bytes = reader.u64()?;
        let _map_full_ckey_verified = reader.bit()?;
        let _map_encoded_key_authenticated = reader.bit()?;
        for _ in 0..5 {
            let _ = reader.u64()?;
        }
        let sections = usize::try_from(reader.u32()?).map_err(|_| "coverage section count")?;
        if sections > 16 {
            return Err("coverage section count");
        }
        for _ in 0..sections {
            let _ = reader.u32()?;
            let _ = reader.u32()?;
            let _ = reader.u32()?;
            let _ = reader.bit()?;
        }

        let liquids = usize::try_from(reader.u32()?).map_err(|_| "coverage liquid count")?;
        if liquids > 2 {
            return Err("coverage liquid count");
        }
        for _ in 0..liquids {
            let _ = reader.take(40)?;
        }
        let maps = usize::try_from(reader.u32()?).map_err(|_| "coverage map count")?;
        if maps > 4096 {
            return Err("coverage map count");
        }
        for _ in 0..maps {
            let _ = reader.take(19)?;
        }

        if !reader.bit()? {
            return Err("coverage missing-model policy");
        }
        let mut omissions = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        let mut omission_ranges = [(0, 0); 5];
        let mut list_start = reader.at;
        omissions[0] = reader.list()?;
        omission_ranges[0] = (list_start, reader.at);
        let resource_profile = reader.text()?.to_owned();
        if resource_profile != RESOURCE_PROFILE || !reader.bit()? {
            return Err("coverage resource or missing-WMO policy");
        }
        for (index, list) in omissions.iter_mut().enumerate().skip(1) {
            list_start = reader.at;
            *list = reader.list()?;
            omission_ranges[index] = (list_start, reader.at);
        }
        validate_disjoint(&omissions)?;

        let artifacts = usize::try_from(reader.u32()?).map_err(|_| "coverage artifact count")?;
        if artifacts > 65_535 {
            return Err("coverage artifact count");
        }
        let mut previous: Option<String> = None;
        for _ in 0..artifacts {
            let path = reader.text()?.to_owned();
            if previous.as_ref().is_some_and(|prior| prior >= &path) {
                return Err("coverage artifact order");
            }
            previous = Some(path);
            let _ = reader.u64()?;
            let _ = reader.take(32)?;
        }
        if reader.at != bytes.len() {
            return Err("coverage trailing bytes");
        }
        Ok(Self {
            version: 16,
            coverage_profile,
            source_profile,
            domain_profile,
            resource_profile,
            omissions,
            omission_ranges,
        })
    }
}

fn validate_disjoint(lists: &[Vec<u32>; 5]) -> Result<(), &'static str> {
    for left in 0..lists.len() {
        for right in left + 1..lists.len() {
            if lists[left]
                .iter()
                .any(|id| lists[right].binary_search(id).is_ok())
            {
                return Err("coverage omission overlap");
            }
        }
    }
    Ok(())
}

pub fn canonical_ordered_ids(ids: &[u32]) -> bool {
    ids.iter().all(|id| *id != 0) && ids.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::{LIST_ORDER, canonical_ordered_ids};

    #[test]
    fn omission_id_order_is_canonical() {
        assert_eq!(LIST_ORDER.len(), 5);
        assert!(canonical_ordered_ids(&[1, 2, 3]));
        assert!(!canonical_ordered_ids(&[1, 1]));
        assert!(!canonical_ordered_ids(&[2, 1]));
        assert!(!canonical_ordered_ids(&[0]));
    }
}
