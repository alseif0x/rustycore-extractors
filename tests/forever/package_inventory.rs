// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic deterministic inventory scale QA.
// External helper: no client files, semantic format or runtime acceptance.
use forever_casc::{
    Locale,
    package::{self, Component, Context, Run},
};
use std::path::PathBuf;
fn main() {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    assert!(a.len() == 2 || a.len() == 3);
    let total: u32 = if a.len() == 3 {
        a[2].to_str().unwrap().parse().unwrap()
    } else {
        1024
    };
    assert!(total > 0 && total <= 1024);
    let out = PathBuf::from(&a[0]);
    let reverse = a[1] == "reverse";
    let resume = a[1] == "resume";
    assert!(reverse || resume || a[1] == "forward");
    // Authored synthetic identities; these do not identify a real client build.
    let mut plan_sha256 = [2; 32];
    if total != 1024 {
        plan_sha256[..4].copy_from_slice(&total.to_le_bytes());
    }
    let context = Context {
        build: 70205,
        locale: Locale::EnUs,
        components: 63,
        source_sha256: [1; 32],
        plan_sha256,
        producer_sha256: [3; 32],
    };
    let mut run = if resume {
        Run::resume(&out, context, &[], Default::default())
    } else {
        Run::create(&out, context, &[], Default::default())
    }
    .unwrap();
    for index in 0..total {
        let i = if reverse { total - 1 - index } else { index };
        let c = Component::ALL[(i as usize) % 6];
        let prefix = if c == Component::Db2 {
            "dbc/enUS"
        } else {
            c.name()
        };
        let name = format!("{prefix}/group {:02}/artifact {i:04}.bin", i % 17);
        let bytes = i.to_le_bytes();
        run.put(c, &name, &bytes).unwrap();
    }
    for c in Component::ALL {
        run.finish_component(c).unwrap();
    }
    let summary = run.finish().unwrap();
    assert_eq!(summary.files, total as usize);
    assert_eq!(summary.bytes, u64::from(total) * 4);
    assert_eq!(
        package::verify(&out, Some(context), Default::default()).unwrap(),
        summary
    );
    println!("{}", summary.json());
}
