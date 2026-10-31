// SPDX-License-Identifier: GPL-3.0-or-later

use forever_casc::integrity;

pub const PATH: &str =
    "vmaps/available-gameobject-expanded-material-filtered-wmo-geometry-coverage-v20.bin";
pub const COVERAGE_PROFILE: &str =
    "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20";
pub const SOURCE_PROFILE: &str = "linux70450-gameobject-expanded-material-filtered-wmo-v1";
pub const DOMAIN_PROFILE: &str = "gameobject-expanded-material-filtered-wmo-v1";
pub const RESOURCE_PROFILE: &str = "linux-collision-gameobject-expanded-resources-v1";
pub const V48_PATH: &str =
    "vmaps/available-gameobject-missing-root-material-filtered-wmo-geometry-coverage-v21.bin";
pub const V48_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21";
pub const V48_SOURCE_PROFILE: &str = "linux70450-gameobject-missing-root-material-filtered-wmo-v1";
pub const V48_DOMAIN_PROFILE: &str = "gameobject-missing-root-material-filtered-wmo-v1";
pub const V48_RESOURCE_PROFILE: &str =
    "linux-collision-gameobject-missing-root-expanded-resources-v1";
pub const V48_MISSING_ROOT_MAX: usize =
    forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS;
pub const V49_PATH: &str = "vmaps/available-gameobject-root-encrypted-unavailable-material-filtered-wmo-geometry-coverage-v22.bin";
pub const V49_COVERAGE_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22";
pub const V49_SOURCE_PROFILE: &str =
    "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1";
pub const V49_DOMAIN_PROFILE: &str =
    "gameobject-root-encrypted-unavailable-material-filtered-wmo-v1";
pub const V49_RESOURCE_PROFILE: &str =
    "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1";
pub const V49_MAX: usize = forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS;

pub const LIST_ORDER: [&str; 5] = [
    "missing_m2",
    "missing_wmo",
    "empty_wmo",
    "unselected_wmo",
    "material_filtered_wmo",
];

#[derive(Debug, PartialEq, Eq)]
pub struct GameObjectSection {
    pub ordinal: u32,
    pub records: u32,
    pub copies: u32,
    pub admitted: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GameObjectOutcome {
    pub ordinal: u64,
    pub display_id: u32,
    pub file_data_id: u32,
    pub disposition: u32,
}

pub struct GameObjectFacts {
    pub declared_direct_rows: u64,
    pub declared_copy_records: u64,
    pub direct_rows: u64,
    pub copy_records: u64,
    pub candidate_requests: u64,
    pub direct_outcomes: u64,
    pub zero_fdids: u64,
    pub textures: u64,
    pub m2_no_collision: u64,
    pub wmo_doodad_only: u64,
    pub missing_root_rows: u64,
    pub missing_root_fdids: Vec<u32>,
    pub encrypted_unavailable_rows: u64,
    pub encrypted_unavailable_fdids: Vec<u32>,
    pub collision_rows: u64,
    pub list_rows: u64,
    pub list_m2_rows: u64,
    pub list_wmo_rows: u64,
    pub temp_bytes: u64,
    pub dtree_bytes: u64,
    pub temp_sha: [u8; 32],
    pub dtree_sha: [u8; 32],
    pub sections: Vec<GameObjectSection>,
    pub outcomes: Vec<GameObjectOutcome>,
}

pub struct Coverage {
    pub version: u32,
    pub coverage_profile: String,
    pub source_profile: String,
    pub domain_profile: String,
    pub resource_profile: String,
    pub omissions: [Vec<u32>; 5],
    pub omission_ranges: [(usize, usize); 5],
    pub missing_root_rows_range: Option<(usize, usize)>,
    pub missing_root_list_count_range: Option<(usize, usize)>,
    pub missing_root_list_range: Option<(usize, usize)>,
    pub encrypted_unavailable_rows_range: Option<(usize, usize)>,
    pub encrypted_unavailable_list_count_range: Option<(usize, usize)>,
    pub encrypted_unavailable_list_range: Option<(usize, usize)>,
    pub candidate_requests_range: (usize, usize),
    pub zero_fdids_range: (usize, usize),
    pub textures_range: (usize, usize),
    pub gameobjects: GameObjectFacts,
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

    fn count(&mut self, maximum: usize) -> Result<usize, &'static str> {
        let count = usize::try_from(self.u32()?).map_err(|_| "coverage count")?;
        if count > maximum {
            return Err("coverage count");
        }
        Ok(count)
    }

    fn text(&mut self) -> Result<&'a str, &'static str> {
        let length = self.count(4096)?;
        std::str::from_utf8(self.take(length)?).map_err(|_| "coverage text")
    }

    fn list(&mut self, maximum: usize) -> Result<(Vec<u32>, (usize, usize)), &'static str> {
        let start = self.at;
        let count = self.count(maximum)?;
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
        Ok((values, (start, self.at)))
    }
}

pub fn parse(bytes: &[u8]) -> Result<Coverage, &'static str> {
    parse_version(bytes, 20)
}

pub fn parse_v48(bytes: &[u8]) -> Result<Coverage, &'static str> {
    parse_version(bytes, 21)
}

pub fn parse_v49(bytes: &[u8]) -> Result<Coverage, &'static str> {
    parse_version(bytes, 22)
}

fn parse_version(bytes: &[u8], expected_version: u32) -> Result<Coverage, &'static str> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.take(8)? != b"RCVGEO1\0" || reader.u32()? != expected_version {
        return Err("coverage header or version");
    }
    let v48 = expected_version == 21;
    let v49 = expected_version == 22;
    let coverage_profile_expected = if v49 {
        V49_COVERAGE_PROFILE
    } else if v48 {
        V48_COVERAGE_PROFILE
    } else {
        COVERAGE_PROFILE
    };
    let source_profile_expected = if v49 {
        V49_SOURCE_PROFILE
    } else if v48 {
        V48_SOURCE_PROFILE
    } else {
        SOURCE_PROFILE
    };
    let domain_profile_expected = if v49 {
        V49_DOMAIN_PROFILE
    } else if v48 {
        V48_DOMAIN_PROFILE
    } else {
        DOMAIN_PROFILE
    };
    let resource_profile_expected = if v49 {
        V49_RESOURCE_PROFILE
    } else if v48 {
        V48_RESOURCE_PROFILE
    } else {
        RESOURCE_PROFILE
    };
    let gameobject_scope_expected = if v49 {
        "available-fdid-direct-root-absent-or-encrypted-unavailable-v1"
    } else if v48 {
        "available-fdid-direct-missing-root-v1"
    } else {
        "available-fdid-direct-v1"
    };
    let model_scope_expected = if v49 {
        "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1"
    } else if v48 {
        "placement-and-gameobject-with-missing-root-v1"
    } else {
        "placement-and-gameobject-v1"
    };
    let context = reader.text()?;
    if !context.contains("70450") || !context.contains("17") {
        return Err("coverage context scope");
    }
    let coverage_profile = reader.text()?.to_owned();
    let source_profile = reader.text()?.to_owned();
    let domain_profile = reader.text()?.to_owned();
    if coverage_profile != coverage_profile_expected
        || source_profile != source_profile_expected
        || domain_profile != domain_profile_expected
    {
        return Err("coverage profile");
    }
    if !reader.bit()?
        || reader.text()? != gameobject_scope_expected
        || reader.text()? != model_scope_expected
        || reader.text()? != "available-section-map-partial-scope-v1"
        || reader.bit()?
        || reader.text()? != gameobject_scope_expected
        || reader.text()? != "none"
        || !reader.bit()?
        || reader.text()? != model_scope_expected
        || reader.text()? != "complete-LiquidMaterial-LiquidType-v1"
        || reader.text()? != "out_of_scope"
        || reader.bit()?
        || reader.bit()?
        || reader.u32()? != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID
        || reader.u64()? == 0
        || reader.u32()? != 0x3ff
    {
        return Err("coverage policy");
    }

    let map_file_data_id = reader.u32()?;
    let map_declared_bytes = reader.u64()?;
    let map_available_bytes = reader.u64()?;
    let map_full_ckey_verified = reader.bit()?;
    let map_encoded_key_authenticated = reader.bit()?;
    for _ in 0..5 {
        reader.u64()?;
    }
    let map_sections = reader.count(16)?;
    for _ in 0..map_sections {
        reader.u32()?;
        reader.u32()?;
        reader.u32()?;
        reader.bit()?;
    }
    if map_file_data_id != forever_casc::collision::world::MAP_FILE_DATA_ID
        || map_declared_bytes == 0
        || map_available_bytes > map_declared_bytes
        || (map_full_ckey_verified && map_available_bytes != map_declared_bytes)
        || (!map_full_ckey_verified && !map_encoded_key_authenticated)
    {
        return Err("coverage Map facts");
    }

    if reader.u32()? != 2 {
        return Err("coverage liquid count");
    }
    for expected in [
        forever_casc::collision::world::MATERIAL_FILE_DATA_ID,
        forever_casc::collision::world::TYPE_FILE_DATA_ID,
    ] {
        if reader.u32()? != expected {
            return Err("coverage liquid table");
        }
        let bytes = reader.u64()?;
        reader.take(4)?;
        for _ in 0..3 {
            reader.u64()?;
        }
        if bytes == 0 {
            return Err("coverage liquid bytes");
        }
    }

    let maps = reader.count(4096)?;
    for _ in 0..maps {
        reader.u32()?;
        let parent = reader.bit()?;
        reader.u32()?;
        reader.u32()?;
        reader.u32()?;
        reader.bit()?;
        reader.bit()?;
        if !parent {
            continue;
        }
    }
    if !reader.bit()? {
        return Err("coverage missing-M2 policy");
    }
    let mut omissions = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut omission_ranges = [(0, 0); 5];
    (omissions[0], omission_ranges[0]) = reader.list(4096)?;
    if reader.text()? != resource_profile_expected || !reader.bit()? {
        return Err("coverage resource profile");
    }
    for index in 1..omissions.len() {
        (omissions[index], omission_ranges[index]) = reader.list(4096)?;
    }
    for left in 0..omissions.len() {
        for right in left + 1..omissions.len() {
            if omissions[left]
                .iter()
                .any(|id| omissions[right].binary_search(id).is_ok())
            {
                return Err("coverage omission overlap");
            }
        }
    }

    if reader.u32()? != forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID {
        return Err("coverage GODI file data ID");
    }
    let _declared_bytes = reader.u64()?;
    let _available_bytes = reader.u64()?;
    let _full_ckey_verified = reader.bit()?;
    let _encoded_key_authenticated = reader.bit()?;
    let mut counts = [0_u64; 19];
    let mut candidate_requests_range = (0, 0);
    let mut zero_fdids_range = (0, 0);
    let mut textures_range = (0, 0);
    for (index, value) in counts.iter_mut().enumerate() {
        match index {
            5 => candidate_requests_range = (reader.at, reader.at + 8),
            7 => zero_fdids_range = (reader.at, reader.at + 8),
            8 => textures_range = (reader.at, reader.at + 8),
            _ => {}
        }
        *value = reader.u64()?;
    }

    let missing_root_rows_range = (v48 || v49).then(|| (reader.at, reader.at + 8));
    let missing_root_rows = if v48 || v49 { reader.u64()? } else { 0 };
    let mut missing_root_fdids = Vec::new();
    let mut missing_root_list_range = None;
    let missing_root_list_count_range = if v48 || v49 {
        let start = reader.at;
        let count = reader.count(V48_MISSING_ROOT_MAX)?;
        missing_root_fdids.reserve(count);
        for _ in 0..count {
            missing_root_fdids.push(reader.u32()?);
        }
        if missing_root_fdids.contains(&0)
            || missing_root_fdids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("coverage missing-root canonicality");
        }
        missing_root_list_range = Some((start + 4, reader.at));
        Some((start, start + 4))
    } else {
        None
    };
    let encrypted_unavailable_rows_range = v49.then(|| (reader.at, reader.at + 8));
    let encrypted_unavailable_rows = if v49 { reader.u64()? } else { 0 };
    let mut encrypted_unavailable_fdids = Vec::new();
    let mut encrypted_unavailable_list_range = None;
    let encrypted_unavailable_list_count_range = if v49 {
        let start = reader.at;
        let count = reader.count(V49_MAX)?;
        encrypted_unavailable_fdids.reserve(count);
        for _ in 0..count {
            encrypted_unavailable_fdids.push(reader.u32()?);
        }
        if encrypted_unavailable_fdids.contains(&0)
            || encrypted_unavailable_fdids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || missing_root_fdids
                .iter()
                .any(|id| encrypted_unavailable_fdids.binary_search(id).is_ok())
            || (v49
                && omissions.iter().any(|list| {
                    list.iter().any(|id| {
                        missing_root_fdids.binary_search(id).is_ok()
                            || encrypted_unavailable_fdids.binary_search(id).is_ok()
                    })
                }))
        {
            return Err("coverage encrypted-unavailable canonicality");
        }
        encrypted_unavailable_list_range = Some((start + 4, reader.at));
        Some((start, start + 4))
    } else {
        None
    };
    let temp_sha = reader
        .take(32)?
        .try_into()
        .map_err(|_| "coverage temp SHA")?;
    let dtree_sha = reader
        .take(32)?
        .try_into()
        .map_err(|_| "coverage Dtree SHA")?;
    for _ in 0..3 {
        reader.u64()?;
    }
    let section_count = reader.count(16)?;
    let mut sections = Vec::with_capacity(section_count);
    for _ in 0..section_count {
        sections.push(GameObjectSection {
            ordinal: reader.u32()?,
            records: reader.u32()?,
            copies: reader.u32()?,
            admitted: reader.bit()?,
        });
    }
    let outcome_count = reader.count(forever_casc::collision::gameobjects::MAX_DIRECT_ROWS)?;
    let mut outcomes = Vec::with_capacity(outcome_count);
    let mut dispositions = [0_u64; 7];
    for expected_ordinal in 0..outcome_count {
        let outcome = GameObjectOutcome {
            ordinal: reader.u64()?,
            display_id: reader.u32()?,
            file_data_id: reader.u32()?,
            disposition: reader.u32()?,
        };
        if outcome.ordinal != expected_ordinal as u64
            || outcome.disposition
                > if v49 {
                    6
                } else if v48 {
                    5
                } else {
                    4
                }
            || (outcome.file_data_id == 0) != (outcome.disposition == 0)
            || (outcome.disposition == 5 && !v48 && !v49)
            || (outcome.disposition == 6 && !v49)
            || (v49
                && outcome.disposition == 5
                && encrypted_unavailable_fdids
                    .binary_search(&outcome.file_data_id)
                    .is_ok())
        {
            return Err("coverage GODI outcome canonicality");
        }
        dispositions[outcome.disposition as usize] += 1;
        outcomes.push(outcome);
    }
    let mut declared_rows = 0_u64;
    let mut declared_copies = 0_u64;
    let mut admitted_rows = 0_u64;
    let mut admitted_copies = 0_u64;
    for (expected_ordinal, section) in sections.iter().enumerate() {
        if section.ordinal != expected_ordinal as u32 {
            return Err("coverage GODI section canonicality");
        }
        declared_rows += u64::from(section.records);
        declared_copies += u64::from(section.copies);
        if section.admitted {
            admitted_rows += u64::from(section.records);
            admitted_copies += u64::from(section.copies);
        }
    }
    if declared_rows != counts[0]
        || declared_copies != counts[1]
        || admitted_rows != counts[2]
        || admitted_copies != counts[3]
        || counts[2] != outcomes.len() as u64
        || counts[6] != outcomes.len() as u64
        || (v49
            && outcomes
                .iter()
                .filter(|outcome| outcome.file_data_id != 0)
                .count() as u64
                != counts[5])
        || (v49 && counts[5] > V49_MAX as u64)
        || dispositions
            != [
                counts[7],
                counts[8],
                counts[11],
                counts[9],
                counts[10],
                missing_root_rows,
                encrypted_unavailable_rows,
            ]
        || (v49
            && (omissions.iter().any(|list| {
                list.iter().any(|id| {
                    missing_root_fdids.binary_search(id).is_ok()
                        || encrypted_unavailable_fdids.binary_search(id).is_ok()
                })
            }) || outcomes.iter().any(|outcome| {
                (outcome.disposition == 5
                    && encrypted_unavailable_fdids
                        .binary_search(&outcome.file_data_id)
                        .is_ok())
                    || (outcome.disposition == 6
                        && missing_root_fdids
                            .binary_search(&outcome.file_data_id)
                            .is_ok())
            })))
        || ((v48 || v49)
            && outcomes
                .iter()
                .filter(|outcome| outcome.disposition == 5)
                .map(|outcome| outcome.file_data_id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                != missing_root_fdids)
        || (v49
            && outcomes
                .iter()
                .filter(|outcome| outcome.disposition == 6)
                .map(|outcome| outcome.file_data_id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                != encrypted_unavailable_fdids)
        || counts[11] != counts[12]
        || counts[12] != counts[13] + counts[14]
        || counts[17] == 0
        || counts[18] == 0
    {
        return Err("coverage GODI facts");
    }

    let artifacts = reader.count(65_535)?;
    for _ in 0..artifacts {
        reader.text()?;
        reader.u64()?;
        reader.take(32)?;
    }
    if reader.at != bytes.len() {
        return Err("coverage trailing bytes");
    }
    Ok(Coverage {
        version: expected_version,
        coverage_profile,
        source_profile,
        domain_profile,
        resource_profile: resource_profile_expected.to_owned(),
        omissions,
        omission_ranges,
        missing_root_rows_range,
        missing_root_list_count_range,
        missing_root_list_range,
        encrypted_unavailable_rows_range,
        encrypted_unavailable_list_count_range,
        encrypted_unavailable_list_range,
        candidate_requests_range,
        zero_fdids_range,
        textures_range,
        gameobjects: GameObjectFacts {
            declared_direct_rows: counts[0],
            declared_copy_records: counts[1],
            direct_rows: counts[2],
            copy_records: counts[3],
            candidate_requests: counts[5],
            direct_outcomes: counts[6],
            zero_fdids: counts[7],
            textures: counts[8],
            m2_no_collision: counts[9],
            wmo_doodad_only: counts[10],
            missing_root_rows,
            missing_root_fdids,
            encrypted_unavailable_rows,
            encrypted_unavailable_fdids,
            collision_rows: counts[11],
            list_rows: counts[12],
            list_m2_rows: counts[13],
            list_wmo_rows: counts[14],
            temp_bytes: counts[17],
            dtree_bytes: counts[18],
            temp_sha,
            dtree_sha,
            sections,
            outcomes,
        },
    })
}

pub fn replace_v48_missing_root_count(
    bytes: &[u8],
    row_count: u64,
    ids: &[u32],
) -> Result<Vec<u8>, &'static str> {
    let parsed = parse_v48(bytes)?;
    let rows_range = parsed
        .missing_root_rows_range
        .ok_or("missing-root row range")?;
    let count_range = parsed
        .missing_root_list_count_range
        .ok_or("missing-root list count range")?;
    let list_range = parsed
        .missing_root_list_range
        .ok_or("missing-root list range")?;
    if ids.len() > V48_MISSING_ROOT_MAX.saturating_add(1)
        || ids.contains(&0)
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("missing-root list boundary");
    }
    let count = u32::try_from(ids.len()).map_err(|_| "missing-root list count")?;
    let mut output = bytes.to_vec();
    output[rows_range.0..rows_range.1].copy_from_slice(&row_count.to_le_bytes());
    output[count_range.0..count_range.1].copy_from_slice(&count.to_le_bytes());
    let mut replacement = Vec::with_capacity(ids.len() * 4);
    for id in ids {
        replacement.extend_from_slice(&id.to_le_bytes());
    }
    output.splice(list_range.0..list_range.1, replacement);
    Ok(output)
}

pub fn replace_v49_count(
    bytes: &[u8],
    range: (usize, usize),
    count: u64,
) -> Result<Vec<u8>, &'static str> {
    let parsed = parse_v49(bytes)?;
    if range.1.checked_sub(range.0) != Some(8) || range.1 > bytes.len() {
        return Err("coverage counter range");
    }
    if ![
        parsed.candidate_requests_range,
        parsed.zero_fdids_range,
        parsed.textures_range,
        parsed.missing_root_rows_range.unwrap_or((0, 0)),
        parsed.encrypted_unavailable_rows_range.unwrap_or((0, 0)),
    ]
    .contains(&range)
    {
        return Err("coverage counter range");
    }
    let mut output = bytes.to_vec();
    output[range.0..range.1].copy_from_slice(&count.to_le_bytes());
    Ok(output)
}

pub fn replace_v49_candidate_requests(bytes: &[u8], count: u64) -> Result<Vec<u8>, &'static str> {
    let parsed = parse_v49(bytes)?;
    replace_v49_count(bytes, parsed.candidate_requests_range, count)
}

pub fn replace_v49_outcome_lists(
    bytes: &[u8],
    missing_ids: &[u32],
    encrypted_ids: &[u32],
) -> Result<Vec<u8>, &'static str> {
    let parsed = parse_v49(bytes)?;
    let missing_range = parsed
        .missing_root_list_range
        .ok_or("missing-root list range")?;
    let missing_count_range = parsed
        .missing_root_list_count_range
        .ok_or("missing-root list count range")?;
    let encrypted_range = parsed
        .encrypted_unavailable_list_range
        .ok_or("encrypted-unavailable list range")?;
    let encrypted_count_range = parsed
        .encrypted_unavailable_list_count_range
        .ok_or("encrypted-unavailable list count range")?;
    for ids in [missing_ids, encrypted_ids] {
        if ids.len() > V49_MAX || ids.contains(&0) || ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("outcome list boundary");
        }
    }
    if missing_ids
        .iter()
        .any(|id| encrypted_ids.binary_search(id).is_ok())
    {
        return Err("outcome list overlap");
    }
    let mut output = bytes.to_vec();
    for (range, ids) in [
        (encrypted_count_range, encrypted_ids),
        (missing_count_range, missing_ids),
    ] {
        let count = u32::try_from(ids.len()).map_err(|_| "outcome list count")?;
        output[range.0..range.1].copy_from_slice(&count.to_le_bytes());
    }
    for (range, ids) in [
        (encrypted_range, encrypted_ids),
        (missing_range, missing_ids),
    ] {
        let mut replacement = Vec::with_capacity(ids.len() * 4);
        for id in ids {
            replacement.extend_from_slice(&id.to_le_bytes());
        }
        output.splice(range.0..range.1, replacement);
    }
    Ok(output)
}

pub fn replace_omission(
    bytes: &[u8],
    list_index: usize,
    ids: &[u32],
) -> Result<Vec<u8>, &'static str> {
    if list_index >= LIST_ORDER.len()
        || ids.len() > 4097
        || ids.contains(&0)
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("coverage omission boundary");
    }
    let parsed = parse(bytes).or_else(|_| parse_v49(bytes))?;
    let (start, end) = parsed.omission_ranges[list_index];
    let capacity = bytes
        .len()
        .checked_sub(end - start)
        .and_then(|length| length.checked_add(4 + ids.len() * 4))
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

pub fn replace_omissions(bytes: &[u8], lists: &[Vec<u32>; 5]) -> Result<Vec<u8>, &'static str> {
    let current = parse(bytes).or_else(|_| parse_v49(bytes))?;
    let mut output = bytes.to_vec();
    for index in (0..LIST_ORDER.len()).rev() {
        if lists[index] != current.omissions[index] {
            output = replace_omission(&output, index, &lists[index])?;
        }
    }
    Ok(output)
}

pub fn replace_v49_outcome_omissions(
    bytes: &[u8],
    list_index: usize,
    ids: &[u32],
) -> Result<Vec<u8>, &'static str> {
    let parsed = parse_v49(bytes)?;
    let (start, end) = *parsed
        .omission_ranges
        .get(list_index)
        .ok_or("coverage omission index")?;
    if ids.len() > V49_MAX || ids.contains(&0) || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("coverage omission boundary");
    }
    let mut output = bytes.to_vec();
    let mut replacement = Vec::with_capacity(ids.len() * 4);
    for id in ids {
        replacement.extend_from_slice(&id.to_le_bytes());
    }
    output.splice(start..end, replacement);
    Ok(output)
}

pub fn reconstructed_temp_sha(tree: &forever_vmap::gameobject_models::Dtree<'_>) -> [u8; 32] {
    let mut bytes = forever_vmap::gameobject_models::TEMP_MAGIC.to_vec();
    for row in tree.rows() {
        bytes.extend_from_slice(&row.display_id().to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(row.name().len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(row.name());
    }
    integrity::digest(&bytes)
}

pub fn dtree_sha(bytes: &[u8]) -> [u8; 32] {
    integrity::digest(bytes)
}
