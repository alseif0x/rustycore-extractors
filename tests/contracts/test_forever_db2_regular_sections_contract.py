# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-12: explicit numeric regular-section source boundaries.
"""Static source/contract consistency, not executed decoder/native acceptance."""
import ast
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


def compact(source):
    return re.sub(r',(?=[)}])', '', re.sub(r'\s+', '', source))


def rust_integer(source, name):
    match = re.search(r'\bconst\s+' + name + r'\s*:\s*(?:usize|u32)\s*=\s*([^;]+);', source)
    if not match:
        raise AssertionError('missing integer constant ' + name)

    def value(node):
        if isinstance(node, ast.Constant) and type(node.value) is int:
            return node.value
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mult):
            return value(node.left) * value(node.right)
        raise AssertionError('unsupported integer constant expression ' + name)

    return value(ast.parse(match.group(1).strip(), mode='eval').body)


class Db2RegularSectionsContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-db2-regular-sections-contract-v1.json'))
        cls.records = read('crates/forever-db2/src/records.rs')
        cls.module = read('crates/forever-db2/src/records/regular_sections.rs')
        cls.columns = read('crates/forever-db2/src/records/columns.rs')
        cls.units = read('crates/forever-db2/src/records/regular_sections/tests.rs')
        cls.doc = ' '.join(read('docs/forever-db2-regular-sections.md').split())

    def test_separate_versioned_explicit_library_api(self):
        self.assertEqual(self.c['profile'], 'forever-db2-regular-sections-numeric-v1')
        self.assertEqual(self.c['api'], 'DecodedTable::read_regular_sections')
        self.assertEqual(self.c['activation'], 'explicit_supplied_byte_library_api_only')
        source = compact(self.records)
        self.assertIn('pubconstREGULAR_SECTIONS_PROFILE:&\'staticstr="' + self.c['profile'] + '";', source)
        self.assertIn('pubfnread_regular_sections(data:&[u8],table:CharacterTable,max_sections:u32)', source)
        self.assertIn('regular_sections::read(data,table,max_sections)', source)
        self.assertEqual(self.c['wire_format'], 'WDC5')

    def test_ordinary_single_section_policy_remains_explicit(self):
        self.assertTrue(self.c['ordinary_decode_unchanged'])
        source = compact(self.records)
        self.assertIn('ifmeta.sections.len()>1{returnErr(RecordError::Unsupported);}', source)
        ordinary = source.split('fnscan_regular_or_logical(', 1)[1]
        self.assertNotIn('regular_sections::', ordinary)
        self.assertIn('let(cols,metadata_end)=columns(data,&h)?;', ordinary)
        self.assertIn('Self::read_impl(data,table,None,TEXT_POLICY_LIMIT)', source)

    def test_legacy_column_prefix_is_preserved_by_separate_helper(self):
        source = compact(self.columns)
        self.assertIn('fncolumns(data:&[u8],h:&Header)', source)
        self.assertIn('columns_after_prefix(data,h,244)', source)
        self.assertIn('fncolumns_after_prefix(data:&[u8],h:&Header,prefix:usize)', source)
        self.assertIn('letstart=prefix.checked_add(offsets_bytes)', source)
        self.assertIn('h.column_meta_sizeasusize!=meta_bytes', source)
        self.assertIn('ifat>data.len(){returnErr(RecordError::Bounds);}', source)

    def test_no_product_or_component_cli_activation(self):
        self.assertFalse(self.c['product_or_component_cli_enabled'])
        for path in ('crates/forever-db2/src/main.rs',
                     'crates/forever-extractor/src/main.rs',
                     'crates/forever-db2/src/records/sparse.rs',
                     'crates/forever-db2/src/records/terrain_sources.rs'):
            self.assertNotRegex(read(path), r'\bread_regular_sections\s*\(')
        self.assertIn('component/product CLIs', self.doc)
        self.assertIn('existing admission policies', self.doc)

    def test_reviewed_schema_and_hash_gates_precede_decode(self):
        self.assertEqual(self.c['schemas'], 'existing_reviewed_non_catalog_numeric_external_id_unparented')
        self.assertTrue(self.c['schema_hash_gates_preserved'])
        source = compact(self.module)
        self.assertIn('if!table.matches(&h){returnErr(RecordError::Schema);}', source)
        self.assertLess(source.index('table.matches(&h)'), source.index('columns_after_prefix('))
        self.assertIn('matches!(table,CharacterTable::Catalog(_))', source)
        self.assertIn('schema.index>=0', source)
        self.assertIn('schema.parent>=0', source)
        self.assertIn('matches!(kind,Kind::String|Kind::StringArray(_))', source)

    def test_closed_unencrypted_regular_admission(self):
        admission = self.c['admission']
        for field in ('tact_declared', 'sparse', 'text', 'parent', 'inline_id', 'catalog_schema'):
            self.assertFalse(admission[field])
        source = compact(self.module)
        for marker in ('h.flags&SPARSE_FLAG!=0', 'h.parent_lookup_count!=0',
                       'h.string_table_size!=0', 'section.tact_declared',
                       'section.string_table_size!=0', 'section.parent_lookup_data_size!=0'):
            self.assertIn(marker, source)
        self.assertNotRegex(self.module, r'\b(?:unsafe|extern)\s*(?:\{|"C")')
        self.assertNotIn('std::fs', self.module)
        self.assertNotIn('std::net', self.module)

    def test_catalog_offset_zero_is_conservative_profile_restriction(self):
        self.assertTrue(self.c['admission']['catalog_count_and_offset_zero'])
        source = compact(self.module)
        self.assertIn('section.catalog_data_count!=0', source)
        self.assertIn('section.catalog_data_offset!=0', source)
        self.assertIn('conservative Rust profile restriction', self.doc)
        self.assertIn('original regular C++ ignores it', self.doc)

    def test_section_ceiling_is_small_and_downward_only(self):
        sections = self.c['sections']
        self.assertEqual((sections['minimum'], sections['maximum']), (2, 16))
        self.assertTrue(sections['caller_limit_downward_only'])
        self.assertEqual(rust_integer(self.records, 'MAX_REGULAR_SECTIONS'), sections['maximum'])
        source = compact(self.module)
        self.assertIn('!(2..=DecodedTable::MAX_REGULAR_SECTIONS).contains(&max_sections)', source)
        self.assertIn('h.section_count>max_sections', source)
        self.assertIn('h.section_count<2', source)
        self.assertLess(source.index('contains(&max_sections)'), source.index('Metadata::read('))

    def test_resource_caps_are_fixed_whole_file_policies(self):
        limits = self.c['limits']
        for constant, field in (('MAX_BYTES', 'input_bytes'), ('MAX_RECORDS', 'direct_records'),
                                ('MAX_COPIES', 'copy_pairs'), ('MAX_VALUES', 'retained_scalar_values')):
            self.assertEqual(rust_integer(self.module, constant), limits[field])
        self.assertEqual(limits['scope'], 'whole_file_cumulative')
        self.assertFalse(limits['rss_bound'])
        source = compact(self.module)
        self.assertIn('read_with_limits(data,table,max_sections,MAX_RECORDS,MAX_COPIES,MAX_VALUES)', source)
        self.assertIn('data.len()>MAX_BYTES', source)
        self.assertIn('record_count.checked_add(section.record_countasusize)', source)
        self.assertIn('copy_count.checked_add(section.copy_table_countasusize)', source)
        self.assertIn('(h.record_countasusize).checked_mul(row_values)', source)
        self.assertIn('value_count.checked_add(row_values)', source)
        self.assertIn('Section count does not multiply those limits', self.doc)
        self.assertIn('not an exact heap or RSS measurement', self.doc)

    def test_complete_metadata_and_exact_contiguous_framing(self):
        for field in ('complete_primitive_columns', 'contiguous_sections', 'exact_id_tables_and_eof'):
            self.assertTrue(self.c['admission'][field])
        source = compact(self.module)
        for marker in ('checked_mul(SECTION_SIZE)', 'HEADER_SIZE.checked_add(bytes)',
                       'columns_after_prefix(data,&h,prefix)?',
                       'section.file_offsetasusize!=end',
                       'section.id_table_sizeasusize!=id_bytes',
                       'record_count!=h.record_countasusize||end!=data.len()',
                       '.filter(|end|*end<=data.len())'):
            self.assertIn(marker, source)
        self.assertLess(source.index('end!=data.len()'), source.index('letmutrows=BTreeMap::new()'))

    def test_global_repair_ordinal_and_direct_id_validation(self):
        self.assertEqual(self.c['semantics']['id_repair'], 'global_direct_record_ordinal')
        source = compact(self.module)
        self.assertIn('letmutordinal=0u32;', source)
        self.assertIn('ifstored<=h.min_id', source)
        self.assertIn('h.min_id.checked_add(ordinal)', source)
        self.assertIn('ordinal=ordinal.checked_add(1)', source)
        self.assertIn('id>h.max_id||rows.contains_key(&id)', source)
        self.assertLess(source.index('letmutordinal=0u32;'), source.index('forrecordin0..section.record_count'))
        self.assertEqual(source.count('letmutrows=BTreeMap::new()'), 1)

    def test_copy_replay_has_one_global_pass_after_all_direct_records(self):
        semantics = self.c['semantics']
        self.assertEqual(semantics['copies'], 'one_global_ordered_pass_after_all_direct_records')
        self.assertFalse(semantics['forward_copy_retry'])
        self.assertEqual(semantics['self_copy'], 'reject_if_applicable')
        source = compact(self.module)
        for marker in ('max_id=max_id.max(u32_at(data,range.copies+index*8)?)',
                       'max_id.checked_add(1)', 'source==0||source>=extent||new>=extent',
                       'letSome(source_fields)=rows.get(&source)else{continue;}',
                       'ifsource==new{returnErr(RecordError::Unsupported);}',
                       'if!rows.contains_key(&new)', 'letfields=source_fields.clone();',
                       'rows.insert(new,fields);'):
            self.assertIn(marker, source)
        self.assertLess(source.index('rows.insert(id,fields);'), source.index('letextent='))
        self.assertLess(source.index('letextent='), source.index('letSome(source_fields)'))
        self.assertIn('Forward copy references are not retried', self.doc)

    def test_all_or_error_result_and_authored_unit_inventory(self):
        self.assertFalse(self.c['semantics']['partial_result'])
        source = compact(self.module)
        self.assertEqual(source.count('Ok(DecodedTable{rows})'), 1)
        self.assertLess(source.index('rows.insert(new,fields);'), source.index('Ok(DecodedTable{rows})'))
        names = re.findall(r'#\[test\]\s+fn\s+([a-z0-9_]+)\s*\(', self.units)
        self.assertEqual(len(names), self.c['source_checkpoint']['authored_unit_cases'])
        self.assertEqual(len(names), len(set(names)))
        self.assertNotIn('#[ignore', self.units)
        for name in ('empty_middle_section_keeps_global_id_repair_ordinal',
                     'copies_see_later_directs_then_global_chains_and_overwrites',
                     'cumulative_direct_copy_and_scalar_limits_are_not_per_section',
                     'legacy_entry_points_still_reject_multi_section_files',
                     'catalog_schemas_are_not_silently_admitted'):
            self.assertIn(name, names)

    def test_authored_checkpoint_does_not_claim_execution_or_completion(self):
        checkpoint = self.c['source_checkpoint']
        for field in ('unit_execution_accepted', 'native_materialization_accepted',
                      'real_client_accepted', 'full_extractor_complete'):
            self.assertFalse(checkpoint[field])
        for marker in ('authored synthetic unit cases',
                       'not yet been executed at this checkpoint',
                       'No new passing regression, native byte comparison or real-client compatibility',
                       '02245dcd245e7433e524577656177723d3e4992e',
                       'AutoProduceData', 'AutoProduceRecordCopies',
                       'Malformed and policy cases belong to Rust-only rejection evidence',
                       'does not repair encrypted mandatory inputs',
                       'complete the extractor'):
            self.assertIn(marker, self.doc)


if __name__ == '__main__':
    unittest.main()
