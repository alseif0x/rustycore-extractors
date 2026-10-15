// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
const V13: &str = "forever-product\t13\nbuild\t70374\nlocale\tesES\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70374-v1\ndecryption\tdisabled\ncomponents\tmaps\nterrain\tall-available-maps\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";
#[test]
fn v13_canonical_full_map_and_available_domain() {
    assert_eq!(
        include_str!("../../../../tests/contracts/fixtures/forever-product-v13/available.tsv"),
        V13
    );
    let map_example =
        include_str!("../../../../tests/contracts/fixtures/forever-product-v13/map0.tsv");
    assert_eq!(
        Plan::parse(map_example).unwrap().render(),
        V13.replace("all-available-maps", "map:0")
    );
    let p = Plan::parse(V13).unwrap();
    assert!(p.is_v13());
    assert_eq!(p.version(), 13);
    assert_eq!(p.components, 8);
    assert_eq!(p.contract(), "forever-product-v13");
    assert_eq!(p.render(), V13);
    let q = Plan {
        terrain: Selection::Map(u32::MAX),
        pack: false,
        locale: Locale::EnUs,
        ..p
    };
    assert_eq!(Plan::parse(&q.render()), Ok(q));
}
#[test]
fn v13_closed_components_build_catalog_and_key_policy() {
    let p = Plan::parse(V13).unwrap();
    for components in 0..=255 {
        if components != 8 {
            assert!(
                Plan {
                    components,
                    ..p.clone()
                }
                .validate()
                .is_err()
            );
        }
    }
    for build in [0, 70170, 70205, 70338] {
        assert!(Plan { build, ..p.clone() }.validate().is_err());
    }
    assert!(
        Plan {
            schema_catalog: SchemaCatalog::PinnedMetadataV1,
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            navigation: Selection::All,
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            map_domain: Some(2),
            ..p.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            offmesh: Some(vec![]),
            ..p
        }
        .validate()
        .is_err()
    );
}
#[test]
fn v13_rejects_tile_none_aliases_and_legacy_relabeling() {
    for word in ["all", "none", "tile:0:31:31", "map:00"] {
        assert!(Plan::parse(&V13.replace("all-available-maps", word)).is_err());
    }
    let p = Plan::parse(V13).unwrap();
    assert!(
        Plan {
            publication_profile: PublicationProfile::Legacy,
            ..p
        }
        .validate()
        .is_err()
    );
    assert!(Plan::parse(&V13.replace("forever-product\t13", "forever-product\t11")).is_err());
}
#[test]
fn v13_rejects_before_any_client_input() {
    #[cfg(target_os = "linux")]
    {
        let mut p = Plan::parse(V13).unwrap();
        p.decryption = Decryption::ProvidedSalsa20V1;
        assert_eq!(
            crate::driver::open(&p, std::path::Path::new("/authored/absent/client"))
                .err()
                .unwrap()
                .code,
            "product_plan_dependencies"
        );
    }
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        crate::driver::admit(&Plan::parse(V13).unwrap())
            .err()
            .unwrap()
            .code,
        "product_platform"
    );
}
