// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit synthetic pipeline corpus and real QA executable.
use super::*;
use std::{fs, path::Path, process::Command};

#[test]
#[ignore = "requires explicit external case directory and built terrain_local QA binary"]
fn synthetic_pipeline_native_corpus_and_cli() {
    let root = std::path::PathBuf::from(
        std::env::var_os("RCE_TERRAIN_CASES").expect("explicit corpus output"),
    );
    let cli = std::env::var_os("RCE_TERRAIN_QA_BIN").expect("explicit built QA binary");
    let parent = fs::canonicalize(root.parent().unwrap()).unwrap();
    assert!(!parent.ancestors().any(|p| p.join(".git").exists()));
    assert!(!root.exists());
    fs::create_dir(&root).unwrap();
    let mut inventory = String::new();
    let mut case_no = 0;
    for build in [70170, 70205] {
        for pack in [false, true] {
            for variant in 0..5 {
                let mut files = basic();
                if matches!(variant, 1 | 2) {
                    files[1].2 = material_db2(&[(5, if variant == 2 { 2 } else { 0 })], &[(7, 5)]);
                    files[3].2 = type_db2(&[(21, 7)], &[(22, 21)]);
                    files[5].2 = liquid_adt();
                }
                if variant == 3 {
                    files[4].2 = wdt(&[(3, 2, 999), (1, 4, 999)], false);
                    files[5].1 = Some(b"World/Maps/Synthetic/Synthetic_3_2.adt");
                    files[6].1 = Some(b"world\\maps\\synthetic\\synthetic_1_4.adt");
                }
                if variant == 4 {
                    files[0].2 = map_db2(&[(7, 100, -1)], &[(8, 7), (9, 8)]);
                }
                let fixture = CollisionFixture::for_build(&files, build);
                let mut stage = Stage::default();
                let complete = run(
                    &mut fixture.open(variant == 3),
                    Selection::AllMaps,
                    pack,
                    Limits::default(),
                    &mut stage,
                )
                .unwrap();
                assert_eq!(complete.plan().maps, if variant == 4 { 2 } else { 1 });
                assert_eq!(
                    stage
                        .tiles
                        .iter()
                        .map(|t| t.1.file_data_id)
                        .collect::<Vec<_>>(),
                    if variant == 4 {
                        vec![201, 202, 201, 202]
                    } else {
                        vec![201, 202]
                    }
                );
                let dir = root.join(format!("case-{case_no:02}"));
                fs::create_dir(&dir).unwrap();
                let lookup = lookup(variant);
                fs::write(dir.join("lookup"), &lookup).unwrap();
                for (index, (context, facts, maps, _)) in stage.tiles.iter().enumerate() {
                    assert_eq!(
                        (context.tile_x, context.tile_y),
                        if index % 2 == 0 { (3, 2) } else { (1, 4) }
                    );
                    assert_eq!(context.map_id, if index < 2 { 7 } else { 8 });
                    let source = &files.iter().find(|f| f.0 == facts.file_data_id).unwrap().2;
                    fs::write(dir.join(format!("adt-{index}")), source).unwrap();
                    fs::write(dir.join(format!("maps-{index}")), maps).unwrap();
                    let mut ctx = b"RCECONV1".to_vec();
                    ctx.extend(context.map_id.to_le_bytes());
                    ctx.extend([context.tile_y, context.tile_x]);
                    ctx.extend(build.to_le_bytes());
                    ctx.push(u8::from(pack));
                    for n in [16 * 1024 * 1024u32, 4096, 202112] {
                        ctx.extend(n.to_le_bytes());
                    }
                    fs::write(dir.join(format!("context-{index}")), ctx).unwrap();
                    inventory.push_str(&format!("map\tcase-{case_no:02}\t{index}\n"));
                }
                for (index, (map, list)) in stage.lists.iter().enumerate() {
                    let coords: Vec<_> = stage
                        .tiles
                        .iter()
                        .filter(|t| t.0.map_id == *map)
                        .map(|t| [t.0.tile_x, t.0.tile_y])
                        .collect();
                    let mut input = b"RCEBTL01".to_vec();
                    input.extend(build.to_le_bytes());
                    input.extend((coords.len() as u32).to_le_bytes());
                    for pair in coords {
                        input.extend(pair);
                    }
                    fs::write(dir.join(format!("bits-{index}")), input).unwrap();
                    fs::write(dir.join(format!("list-{index}")), list).unwrap();
                    inventory.push_str(&format!("list\tcase-{case_no:02}\t{index}\n"));
                }
                let dest = dir.join("cli");
                let mut command = Command::new(&cli);
                command
                    .arg(fixture.path())
                    .arg("all")
                    .arg(&dest)
                    .arg("--locale=enUS");
                if variant == 3 {
                    command.arg("--names");
                }
                if !pack {
                    command.arg("--no-pack");
                }
                let output = command.output().unwrap();
                fs::write(dir.join("cli.stdout"), &output.stdout).unwrap();
                fs::write(dir.join("cli.stderr"), &output.stderr).unwrap();
                assert!(
                    output.status.success(),
                    "QA CLI failed in synthetic case {case_no}"
                );
                for (c, _, bytes, _) in &stage.tiles {
                    assert_eq!(
                        fs::read(
                            dest.join(
                                plan::map_relative_path(c.map_id, c.tile_x, c.tile_y).unwrap()
                            )
                        )
                        .unwrap(),
                        *bytes
                    );
                }
                for (map, bytes) in &stage.lists {
                    assert_eq!(
                        fs::read(dest.join(format!("maps/{map:04}.tilelist"))).unwrap(),
                        *bytes
                    );
                }
                let summary = fs::read(dest.join("complete.json")).unwrap();
                let result = Command::new(&cli)
                    .arg(fixture.path())
                    .arg("all")
                    .arg(&dest)
                    .output()
                    .unwrap();
                assert_eq!(result.status.code(), Some(4));
                assert_eq!(fs::read(dest.join("complete.json")).unwrap(), summary);
                case_no += 1;
            }
        }
    }
    assert_eq!(case_no, 20);
    fs::write(root.join("inventory.tsv"), inventory).unwrap();
    // Rejection must create no output, including for an existing client root.
    let fixture = CollisionFixture::new(&basic());
    let under_client = fixture.path().join("must-not-write");
    let out = Command::new(&cli)
        .arg(fixture.path())
        .arg("all")
        .arg(&under_client)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(!under_client.exists());
    let protected = root.join("git-boundary");
    fs::create_dir(&protected).unwrap();
    fs::create_dir(protected.join(".git")).unwrap();
    let out = Command::new(&cli)
        .arg(fixture.path())
        .arg("all")
        .arg(protected.join("out"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(!protected.join("out").exists());
    let mut bad = basic();
    bad[0].2.pop();
    let fixture = CollisionFixture::new(&bad);
    let out = Command::new(&cli)
        .arg(fixture.path())
        .arg("all")
        .arg(root.join("failed-source"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert!(!root.join("failed-source").exists());
    fs::write(
        root.join("rust-complete"),
        b"20 synthetic positive CLI cases; 20 exclusivity guards; 3 negative guards\n",
    )
    .unwrap();
    assert!(Path::new(&cli).is_file());
}
fn lookup(variant: u32) -> Vec<u8> {
    let types = if matches!(variant, 1 | 2) {
        vec![(21u32, 0u8, 7u8), (22, 0, 7)]
    } else {
        vec![(22, 0, 7)]
    };
    let materials = if matches!(variant, 1 | 2) {
        vec![
            (5u32, if variant == 2 { 2u32 } else { 0 }, 0i8),
            (7, if variant == 2 { 2 } else { 0 }, 0),
        ]
    } else {
        vec![(7, 0, 0)]
    };
    let mut bytes = b"RCELIQ01".to_vec();
    bytes.extend((types.len() as u32).to_le_bytes());
    bytes.extend((materials.len() as u32).to_le_bytes());
    for (id, sound, material) in types {
        bytes.extend(id.to_le_bytes());
        bytes.extend([sound, material]);
    }
    for (id, flags, lvf) in materials {
        bytes.extend(id.to_le_bytes());
        bytes.extend(flags.to_le_bytes());
        bytes.push(lvf as u8);
    }
    bytes
}
