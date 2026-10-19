// SPDX-License-Identifier: GPL-3.0-or-later
// Shared literal-shape synthetic WDC5 fixture for the build-70450 sparse
// collectable tables. It is derived from public catalogue metadata and the
// observed header structure; it contains no client bytes or record payloads.

macro_rules! define_synthetic_sparse_fixture {
    ($db2:path) => {
        use $db2::{HEADER_SIZE, catalog};

        fn put16(bytes: &mut [u8], offset: usize, value: u16) {
            bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }

        fn put32(bytes: &mut [u8], offset: usize, value: u32) {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }

        fn field_bits(field: catalog::FieldMetadata) -> u32 {
            let width = match field.kind {
                catalog::FieldType::String | catalog::FieldType::NonLocalizedString => 32,
                catalog::FieldType::Byte => 8,
                catalog::FieldType::Short => 16,
                catalog::FieldType::Int | catalog::FieldType::Float => 32,
                catalog::FieldType::Long => 64,
            };
            width * u32::from(field.array_size)
        }

        /// Build an empty sectionless parented sparse file from public
        /// catalogue facts. No client contents are embedded.
        pub fn empty_parented_sparse_70450(name: &str) -> Vec<u8> {
            assert!(matches!(
                name,
                "CollectableSourceEncounterSparse"
                    | "CollectableSourceQuestSparse"
                    | "CollectableSourceVendorSparse"
            ));
            let declaration = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
                .unwrap()
                .unwrap()
                .metadata();
            assert_eq!(
                declaration.parent_index,
                declaration.file_field_count as i16
            );
            assert_eq!(declaration.file_field_count + 1, declaration.field_count);

            let columns = &declaration.fields[..declaration.file_field_count];
            let record_bits = columns.iter().copied().map(field_bits).sum::<u32>();
            let record_size = record_bits.div_ceil(8).max(8);
            let mut bytes = vec![0; HEADER_SIZE + declaration.file_field_count * 4];
            bytes[..4].copy_from_slice(b"WDC5");
            put32(&mut bytes, 4, 5);
            put32(&mut bytes, 140, declaration.file_field_count as u32);
            put32(&mut bytes, 144, record_size);
            put32(&mut bytes, 152, declaration.table_hash.unwrap());
            put32(&mut bytes, 156, declaration.layout_hash);
            put16(&mut bytes, 172, 0x7);
            put16(&mut bytes, 174, declaration.index as u16);
            put32(&mut bytes, 176, declaration.file_field_count as u32);
            // The other record/metadata/parent fields and section count remain
            // zero, matching the observed physical structure.
            for (index, field) in columns.iter().enumerate() {
                let unused_bits = match (name, index) {
                    // The reviewed decoded layout stores the second encounter
                    // value in a signed 16-bit physical slot despite its
                    // catalogue field declaration being Int.
                    ("CollectableSourceEncounterSparse", 1) => 16,
                    _ => match field.kind {
                        catalog::FieldType::Byte => 24,
                        catalog::FieldType::Short => 16,
                        _ => 0,
                    },
                };
                put16(&mut bytes, HEADER_SIZE + index * 4, unused_bits);
                put16(&mut bytes, HEADER_SIZE + index * 4 + 2, u16::MAX);
            }
            bytes
        }
    };
}
