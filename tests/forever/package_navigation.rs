// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: external QA adapter for complete synthetic plane fixtures.
// Build outside the checkout with forever-casc/db2/navigation path dependencies.
// No production product/acquisition CLI or real-client input is admitted here.
use forever_casc::package::{self, Component, Context, Run};
use forever_navigation::{
    inventory::fs::{
        Package,
        run::{self, Event, Sink},
    },
    models, pipeline, recast,
};
use std::{fs, io::Read, path::PathBuf};
struct Writer {
    run: Run,
    stop: bool,
}
impl Sink for Writer {
    type Error = package::Error;
    fn emit(&mut self, event: Event<'_>) -> Result<(), Self::Error> {
        match event {
            Event::Global { map_id, bytes } => {
                self.run.put(
                    Component::Navigation,
                    &format!("mmaps/{map_id:04}.mmap"),
                    bytes,
                )?;
                if self.stop {
                    return Err(package::Error::Incomplete);
                }
            }
            Event::Tile {
                map_id,
                tile,
                outcome,
            } => {
                if let pipeline::Outcome::Tile { bytes, .. } = outcome {
                    self.run.put(
                        Component::Navigation,
                        &format!(
                            "mmaps/{map_id:04}_{:02}_{:02}.mmtile",
                            tile.row, tile.column
                        ),
                        bytes,
                    )?;
                }
            }
        }
        Ok(())
    }
}
fn main() {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    assert!(a.len() == 4);
    let input = PathBuf::from(&a[0]);
    let context_file = PathBuf::from(&a[1]);
    let output = PathBuf::from(&a[2]);
    let stop = a[3] == "stop-after-global";
    let resume = a[3] == "resume";
    assert!(stop || resume || a[3] == "complete");
    let mut text = String::new();
    fs::File::open(context_file)
        .unwrap()
        .take(1025)
        .read_to_string(&mut text)
        .unwrap();
    let context = Context::parse(&text).unwrap();
    assert_eq!(context.components, Component::Navigation.bit());
    let p = fs::read_to_string(input.join("plan.txt")).unwrap();
    let mut w = p.split_whitespace();
    fn n<T: std::str::FromStr>(w: &mut std::str::SplitWhitespace<'_>) -> T
    where
        T::Err: std::fmt::Debug,
    {
        w.next().unwrap().parse().unwrap()
    }
    let count: usize = n(&mut w);
    let _: u32 = n(&mut w);
    let _: u8 = n(&mut w);
    let _: u8 = n(&mut w);
    let skip: u8 = n(&mut w);
    let build: u32 = n(&mut w);
    let big: u8 = n(&mut w);
    let slopes = recast::Slopes {
        maximum: n(&mut w),
        non_steep: n(&mut w),
    };
    assert_eq!(n::<usize>(&mut w), 0);
    assert_eq!(n::<usize>(&mut w), 0);
    assert!(skip <= 1 && big <= 1 && count <= 64 && build == context.build);
    let source = forever_db2::terrain_sources::read_navigation_maps(
        &fs::read(input.join("Map.db2")).unwrap(),
    )
    .unwrap();
    assert_eq!(source.direct_records().len(), count);
    for row in source.direct_records() {
        assert_eq!(n::<u32>(&mut w), row.id);
        assert_eq!(n::<u32>(&mut w), i32::from(row.fields.parent_map_id) as u32);
        assert_eq!(
            n::<u32>(&mut w),
            i32::from(row.fields.cosmetic_parent_map_id) as u32
        );
        let _: u8 = n(&mut w);
    }
    assert!(w.next().is_none());
    let options = fs::read(input.join("options.bin")).unwrap();
    assert_eq!(options.len(), 16);
    let words: Vec<_> = options
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v))
        .collect();
    assert_eq!(words[2], 0);
    assert_eq!(words[3], build);
    let mut inputs = Package::open(
        &input.join("maps"),
        &input.join("vmaps"),
        &source,
        build,
        forever_navigation::inventory::Options {
            map: (words[0] != 0).then_some(words[1]),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    let mut writer = Writer {
        run: if resume {
            Run::resume(&output, context, &[&input], Default::default())
        } else {
            Run::create(&output, context, &[&input], Default::default())
        }
        .unwrap(),
        stop,
    };
    let completed = run::execute(
        &mut inputs,
        models::LiquidTypes::new(&[]).unwrap(),
        &[],
        run::Options {
            skip_liquids: skip != 0,
            big_base_unit: big != 0,
            slopes,
            ..Default::default()
        },
        Default::default(),
        &mut writer,
    );
    if stop {
        let failure = completed.unwrap_err();
        assert!(matches!(
            failure.cause,
            run::Cause::Sink(package::Error::Incomplete)
        ));
        assert_eq!(failure.progress.maps, 0);
        assert_eq!(failure.progress.tiles, 0);
        drop(writer);
        assert_eq!(
            package::verify(&output, Some(context), Default::default()).unwrap_err(),
            package::Error::Incomplete
        );
        println!("{{\"status\":\"expected_incomplete\"}}");
        return;
    }
    let completed = completed.unwrap();
    writer.run.finish_component(Component::Navigation).unwrap();
    let summary = writer.run.finish().unwrap();
    assert_eq!(
        summary.files,
        completed.progress.maps + completed.progress.written_tiles
    );
    assert_eq!(summary.bytes, completed.progress.output_bytes);
    assert_eq!(
        package::verify(&output, Some(context), Default::default()).unwrap(),
        summary
    );
    println!("{}", summary.json());
}
