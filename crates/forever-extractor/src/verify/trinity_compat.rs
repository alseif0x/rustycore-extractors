// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-11-01: verify Trinity-compatible available-map omissions.

use crate::{Error, driver::terrain_v20, input, plan::Plan};
use forever_casc::{package::Summary, terrain::available_70450 as available};
use forever_navigation::package_v57::{Coverage57, Output};
use std::{collections::BTreeSet, path::Path};

const TERRAIN_COVERAGE: &str = "maps/compatible-terrain-coverage-v3.tsv";
const NAVIGATION_COVERAGE: &str = "mmaps/available-navigation-v57.tsv";
const TILELIST_BYTES: usize = 4108;
const CONTRACT: &str = "forever-trinity-compat-v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub maps: usize,
    pub planned: usize,
    pub root_absent: usize,
    pub navigation: Summary,
}

impl Report {
    pub fn json(&self) -> String {
        format!(
            "{{\"contract\":\"{CONTRACT}\",\"status\":\"compatible-partial\",\"maps\":{},\"planned\":{},\"root_absent\":{},\"full_world_complete\":false,\"runtime_accepted\":false,\"full_extractor_complete\":false,\"package\":{}}}",
            self.maps,
            self.planned,
            self.root_absent,
            self.navigation.json()
        )
    }
}

pub fn verify_trinity_compat(
    terrain_plan: &Plan,
    terrain_package: &Path,
    navigation_source_plan: &Path,
    navigation_plan: &Plan,
    navigation_package: &Path,
) -> Result<Report, Error> {
    if !terrain_plan.is_v51() || !navigation_plan.is_v57() {
        return Err("product_trinity_compat_profiles".into());
    }
    let terrain_summary = crate::verify::verify_links(terrain_plan, terrain_package, None)?;
    let source_plan_bytes = input::bytes(navigation_source_plan, crate::MAX_PLAN_BYTES)?;
    let source_plan = Plan::parse(
        std::str::from_utf8(&source_plan_bytes).map_err(|_| Error::from("product_plan_format"))?,
    )
    .map_err(Error::from)?;
    require(
        source_plan.is_v51() && source_plan.locale == navigation_plan.locale,
        "product_navigation_input_plan",
    )?;
    let navigation_summary = crate::verify::verify_available_navigation_links(
        navigation_plan,
        terrain_package,
        navigation_source_plan,
        navigation_package,
        None,
    )?;
    let terrain_coverage = terrain_v20::Coverage::parse(&input::bytes(
        &terrain_package.join(TERRAIN_COVERAGE),
        16 * 1024 * 1024,
    )?)?;
    let navigation_coverage = Coverage57::parse(&input::bytes(
        &navigation_package.join(NAVIGATION_COVERAGE),
        128 * 1024 * 1024,
    )?)
    .map_err(|error| Error::from(error.code()))?;
    require(
        terrain_coverage.context == terrain_summary.context
            && navigation_coverage.context == navigation_summary.context
            && navigation_coverage.source == terrain_summary,
        "product_trinity_compat_context",
    )?;
    let terrain_absent: BTreeSet<u32> = terrain_coverage
        .maps
        .iter()
        .filter(|outcome| outcome.wdt == available::WdtDisposition::Unavailable)
        .map(|outcome| outcome.map_id)
        .collect();
    let navigation_absent: BTreeSet<u32> = navigation_coverage
        .maps
        .iter()
        .filter(|map| map.state == "root-absent")
        .map(|map| map.id)
        .collect();
    require(
        !navigation_absent.is_empty() && terrain_absent == navigation_absent,
        "product_trinity_compat_omissions",
    )?;
    for (map, output) in navigation_coverage
        .maps
        .iter()
        .zip(&navigation_coverage.outputs)
    {
        if navigation_absent.contains(&map.id) {
            let tilelist = input::bytes(
                &terrain_package.join(format!("maps/{:04}.tilelist", map.id)),
                TILELIST_BYTES as u64,
            )?;
            check_omission(&tilelist, output)?;
        }
    }
    Ok(Report {
        maps: navigation_coverage.maps.len(),
        planned: navigation_coverage
            .maps
            .iter()
            .filter(|map| map.state == "planned")
            .count(),
        root_absent: navigation_absent.len(),
        navigation: navigation_summary,
    })
}

fn check_omission(tilelist: &[u8], output: &Output) -> Result<(), Error> {
    require(
        tilelist.len() == TILELIST_BYTES && tilelist[12..].iter().all(|byte| *byte == b'0'),
        "product_trinity_compat_tilelist",
    )?;
    require(
        output.global.is_none()
            && output.tiles.is_empty()
            && output.partial_triangulations == 0
            && output.detail_center_fallbacks == 0
            && output.unset_height_samples == 0,
        "product_trinity_compat_navigation",
    )
}

fn require(condition: bool, code: &'static str) -> Result<(), Error> {
    if condition { Ok(()) } else { Err(code.into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forever_navigation::tilebuilder::Coordinate;

    fn output() -> Output {
        Output {
            map: 13,
            global: None,
            tiles: Vec::new(),
            partial_triangulations: 0,
            detail_center_fallbacks: 0,
            unset_height_samples: 0,
        }
    }

    #[test]
    fn empty_tilelist_and_absent_navigation_are_compatible() {
        check_omission(&vec![b'0'; TILELIST_BYTES], &output()).unwrap();
    }

    #[test]
    fn present_tile_or_global_rejects_compatibility() {
        let mut tilelist = vec![b'0'; TILELIST_BYTES];
        tilelist[12] = b'1';
        assert_eq!(
            check_omission(&tilelist, &output()).unwrap_err().code,
            "product_trinity_compat_tilelist"
        );
        let mut with_global = output();
        with_global.global = Some(forever_navigation::package_v57::TileOutput {
            tile: Coordinate { row: 0, column: 0 },
            written: true,
            bytes: 40,
            sha256: [1; 32],
        });
        assert_eq!(
            check_omission(&vec![b'0'; TILELIST_BYTES], &with_global)
                .unwrap_err()
                .code,
            "product_trinity_compat_navigation"
        );
    }

    #[test]
    fn wrong_tilelist_length_rejects_compatibility() {
        assert_eq!(
            check_omission(&vec![b'0'; TILELIST_BYTES - 1], &output())
                .unwrap_err()
                .code,
            "product_trinity_compat_tilelist"
        );
    }
}
