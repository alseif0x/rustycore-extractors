#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Source-only catalog generation/parser checks. Never reads client data."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
import generate_forever_catalog as catalog
sys.path.insert(0, str(ROOT / "tests/forever"))
import db2_oracle as oracle
from test_db2_oracle import Wdc5Builder, _put32, _zero_section_file

# Literal fixtures independently transcribed from the five pinned declarations,
# not constructed from the generated catalog used by the decoder/oracle lookup.
F = oracle.Field
CATALOG_EXTENSION_FIXTURE_SCHEMAS = (
    oracle.Schema('BattlePetEffectProperties', 0xED4FBC95, -1,
                  (F('string', 32, 6), F('unsigned', 16), F('unsigned', 8, 6))),
    oracle.Schema('InitiativeReward', 0x51D55D40, 3,
                  (F('signed', 64, signed=True), F('string'), F('string'), F('unsigned', 32),
                   F('signed', 32, signed=True), F('signed', 32, signed=True), F('signed', 32, signed=True),
                   F('signed', 32, signed=True), F('signed', 32, signed=True))),
    oracle.Schema('LiquidType', 0xD1ECEEC9, -1,
                  (F('string'), F('string', 32, 6), F('signed', 32, signed=True), F('unsigned', 8),
                   F('unsigned', 32), F('unsigned', 32), F('float'), F('float'), F('float'), F('float'),
                   F('unsigned', 16), F('float'), F('unsigned', 8), F('unsigned', 8), F('unsigned', 8),
                   F('signed', 32, signed=True), F('unsigned', 8, 6), F('signed', 32, 3, signed=True),
                   F('float', 32, 38), F('unsigned', 32, 4), F('float', 32, 4))),
    oracle.Schema('TraitCondAccountElement', 0x70915E24, 1,
                  (F('signed', 64, signed=True), F('unsigned', 32), F('signed', 32, signed=True),
                   F('unsigned', 8), F('signed', 32, signed=True), F('signed', 32, signed=True)), parent=2),
    oracle.Schema('UiWidgetVisTypeDataReq', 0x9A097D11, 1,
                  (F('string'), F('unsigned', 32), F('signed', 32, signed=True),
                   F('signed', 32, signed=True), F('signed', 32, signed=True)), parent=5, appended_parent_unsigned=True),
)


def envelope(body):
    return '#ifndef TRINITYCORE_DB2_METADATA_H\n#define TRINITYCORE_DB2_METADATA_H\n#include "DB2Meta.h"\n' + body + '\n#endif\n'


def table(name="Example", file_id=123, field_type="FT_INT", signed="true", count=1):
    return f'''struct {name}Meta
{{
    static constexpr DB2MetaField Fields[1] =
    {{
        {{ .Type = {field_type}, .ArraySize = {count}, .IsSigned = {signed} }},
    }};
    static constexpr DB2Meta Instance =
    {{
        .FileDataId = {file_id},
        .IndexField = -1,
        .ParentIndexField = -1,
        .FieldCount = 1,
        .FileFieldCount = 1,
        .LayoutHash = 0x12345678,
        .Fields = Fields
    }};
}};'''


class ParserTests(unittest.TestCase):
    def reject(self, text, code):
        with self.assertRaisesRegex(catalog.CatalogError, '^' + code + '$'):
            catalog.parse_metadata(text)

    def test_small_declaration_and_comments(self):
        result = catalog.parse_metadata(envelope('/* attribution */\n' + table()))
        self.assertEqual(result, [{
            'name': 'Example', 'file_data_id': 123, 'layout_hash': 0x12345678,
            'index': -1, 'parent_index': -1, 'field_count': 1,
            'file_field_count': 1,
            'fields': [{'type': 'FT_INT', 'array_size': 1, 'signed': True}],
            'decode_blockers': [], 'appended_parent_storage_bits': None,
        }])

    def test_sorted_names(self):
        result = catalog.parse_metadata(envelope(table('Zed') + table('Alpha', 124)))
        self.assertEqual([row['name'] for row in result], ['Alpha', 'Zed'])

    def test_duplicate_name(self):
        self.reject(envelope(table() + table(file_id=124)), 'duplicate_table_name_or_id')

    def test_duplicate_id(self):
        self.reject(envelope(table() + table('Another')), 'duplicate_table_name_or_id')

    def test_unknown_field_type_and_bad_array(self):
        for kind, count in [('FT_INVALID', 1), ('FT_INT', 0), ('FT_INT', 256)]:
            with self.subTest(kind=kind, count=count):
                self.reject(envelope(table(field_type=kind, count=count)), 'invalid_field_type_or_array')

    def test_declared_and_actual_field_counts(self):
        for old, new in [('Fields[1]', 'Fields[2]'), ('.FieldCount = 1', '.FieldCount = 2'), ('.FileFieldCount = 1', '.FileFieldCount = 0')]:
            with self.subTest(new=new):
                self.reject(envelope(table().replace(old, new)), 'invalid_field_counts')

    def test_invalid_id_and_layout_range(self):
        self.reject(envelope(table(file_id=0)), 'invalid_table_identity')
        self.reject(envelope(table(file_id=2**32)), 'invalid_table_identity')
        self.reject(envelope(table().replace('0x12345678', '0x100000000')), 'invalid_table_identity')

    def test_invalid_index_and_parent(self):
        for key in ['IndexField', 'ParentIndexField']:
            for value in ['-2', '1']:
                with self.subTest(key=key, value=value):
                    self.reject(envelope(table().replace(f'.{key} = -1', f'.{key} = {value}')), 'invalid_field_index')

    def test_inline_must_be_unsigned_scalar_at_most_32_bits(self):
        for declaration in [table(), table(field_type='FT_LONG', signed='false'), table(signed='false', count=2)]:
            self.reject(envelope(declaration.replace('.IndexField = -1', '.IndexField = 0')), 'invalid_inline_index_shape')

    def test_parent_must_be_integer_scalar_at_most_32_bits(self):
        for declaration in [table(field_type='FT_STRING'), table(field_type='FT_LONG'), table(count=2)]:
            self.reject(envelope(declaration.replace('.ParentIndexField = -1', '.ParentIndexField = 0')), 'invalid_parent_shape')

    def test_known_unrepresentable_shapes_are_preserved(self):
        for kind, count in [('FT_LONG', 1), ('FT_STRING', 2)]:
            row = catalog.parse_metadata(envelope(table(field_type=kind, count=count)))[0]
            self.assertEqual(row['decode_blockers'], [])
            self.assertEqual(row['fields'][0]['array_size'], count)

    def test_appended_parent_keeps_wire_type_and_normalizes_storage(self):
        for signed, blocked in [('true', []), ('false', [])]:
            declaration = table().replace('Fields[1]', 'Fields[2]')
            declaration = declaration.replace('.FieldCount = 1', '.FieldCount = 2')
            declaration = declaration.replace('.ParentIndexField = -1', '.ParentIndexField = 1')
            declaration = declaration.replace('    };\n    static constexpr DB2Meta Instance',
                f'        {{ .Type = FT_BYTE, .ArraySize = 1, .IsSigned = {signed} }},\n    }};\n    static constexpr DB2Meta Instance')
            row = catalog.parse_metadata(envelope(declaration))[0]
            self.assertEqual(row['fields'][1], {'type': 'FT_BYTE', 'array_size': 1, 'signed': signed == 'true'})
            self.assertEqual(row['decode_blockers'], blocked)
            self.assertEqual(row['appended_parent_storage_bits'], 32)
            self.reject(envelope(declaration.replace('.ParentIndexField = 1', '.ParentIndexField = -1')), 'unsupported_extra_field_shape')

    def test_string_localization_fact_is_retained(self):
        rows = catalog.parse_metadata(envelope(table('Localized', field_type='FT_STRING') + table('NonLocalized', 124, field_type='FT_STRING_NOT_LOCALIZED')))
        self.assertEqual([row['fields'][0]['type'] for row in rows], ['FT_STRING', 'FT_STRING_NOT_LOCALIZED'])

    def test_extra_unknown_instance_member(self):
        self.reject(envelope(table().replace('.Fields = Fields', '.Unexpected = 1, .Fields = Fields')), 'invalid_instance_declaration')

    def test_partial_or_unparsed_declarations_rejected(self):
        for text in [table() + '\nextern int surprise;', 'int surprise;\n' + table(), table().replace('DB2Meta Instance', 'DB2Meta Other')]:
            self.reject(envelope(text), 'unparsed_metadata_remainder')

    def test_unparsed_field_member_rejected(self):
        self.reject(envelope(table().replace('.Type =', '.Extra = 1, .Type =')), 'unparsed_field_remainder')

    def test_empty_missing_envelope_and_tail_rejected(self):
        self.reject(envelope(''), 'unparsed_metadata_remainder')
        self.reject(table(), 'invalid_metadata_envelope')
        self.reject(envelope(table()) + 'unexpected', 'invalid_metadata_envelope')

    def test_hash_mismatch_is_closed(self):
        with self.assertRaisesRegex(catalog.CatalogError, 'pinned_source_mismatch'):
            catalog.document(envelope(table()).encode())


class CommittedCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(catalog.FACTS_PATH.read_text())
        cls.facts = {table['name']: table for table in cls.doc['tables']}

    def test_generated_rust_and_json_are_deterministic(self):
        self.assertEqual(catalog.FACTS_PATH.read_text(), catalog.render_json(self.doc))
        self.assertEqual(catalog.RUST_PATH.read_text(), catalog.render_rust(self.doc))

    def test_catalog_counts_unique_identity_and_pinned_provenance(self):
        self.assertEqual(len(self.doc['tables']), 1126)
        self.assertEqual(len(self.facts), 1126)
        self.assertEqual(len({table['file_data_id'] for table in self.facts.values()}), 1126)
        self.assertEqual(list(self.facts), sorted(self.facts))
        self.assertEqual(sum(not table['decode_blockers'] for table in self.facts.values()), 1126)
        self.assertEqual(self.doc['source']['commit'], catalog.PIN)
        self.assertEqual(self.doc['source']['sha256'], catalog.SOURCE_SHA256)
        self.assertEqual(self.doc['source']['git_blob'], catalog.SOURCE_BLOB)

    def test_all_pinned_shapes_have_value_representations(self):
        self.assertTrue(all(not table['decode_blockers'] for table in self.facts.values()))
        self.assertEqual(self.facts['BattlePetEffectProperties']['fields'][0]['array_size'], 6)
        self.assertEqual(self.facts['InitiativeReward']['fields'][0]['type'], 'FT_LONG')
        self.assertFalse(self.facts['UiWidgetVisTypeDataReq']['fields'][-1]['signed'])

    def test_all_fields_and_parent_normalization(self):
        narrow_parents = []
        for name, table in self.facts.items():
            with self.subTest(table=name):
                self.assertEqual(len(table['fields']), table['field_count'])
                self.assertEqual(table['decode_blockers'], catalog.blockers(table))
                if table['parent_index'] >= table['file_field_count']:
                    self.assertEqual(table['field_count'], table['file_field_count'] + 1)
                    self.assertEqual(table['appended_parent_storage_bits'], 32)
                    parent = table['fields'][-1]
                    if parent['type'] == 'FT_BYTE' and parent['signed']:
                        narrow_parents.append(name)
                else:
                    self.assertIsNone(table['appended_parent_storage_bits'])
        self.assertEqual(narrow_parents, ['CharStartKit', 'ChrClassesXPowerTypes', 'ChrCreateClassAnimTargetInfo', 'ChrCustomization', 'UiCamFbackTalkingHeadChrRace'])

    def test_all_56_independently_reviewed_oracle_physical_fields(self):
        spec = importlib.util.spec_from_file_location('catalog_reviewed_oracle', ROOT / 'tests/forever/db2_oracle.py')
        oracle = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = oracle
        spec.loader.exec_module(oracle)
        self.assertEqual(len(oracle.SCHEMAS), 56)
        for name, reviewed in oracle.SCHEMAS.items():
            with self.subTest(table=name):
                fact = self.facts[name]
                self.assertEqual((fact['layout_hash'], fact['index'], fact['parent_index'], fact['file_field_count']),
                                 (reviewed.layout_hash, reviewed.index, reviewed.parent, len(reviewed.fields)))
                for raw, independent in zip(fact['fields'], reviewed.fields):
                    kind = raw['type']
                    expected_kind = ('string' if kind.startswith('FT_STRING') else 'float' if kind == 'FT_FLOAT' else 'signed' if raw['signed'] else 'unsigned')
                    expected_bits = {'FT_BYTE': 8, 'FT_SHORT': 16, 'FT_INT': 32, 'FT_LONG': 64}.get(kind, 32)
                    self.assertEqual((expected_kind, expected_bits, raw['array_size']), (independent.kind, independent.bits, independent.count))

    @unittest.skipUnless(os.environ.get('RCE_FOREVER_CASC_REFERENCE'), 'pinned local reference not configured')
    def test_full_regeneration_from_verified_local_git_object(self):
        doc = catalog.document(catalog.read_pinned(os.environ['RCE_FOREVER_CASC_REFERENCE']))
        self.assertEqual(doc, self.doc)
        self.assertEqual(catalog.render_rust(doc), catalog.RUST_PATH.read_text())


class CatalogDecoderTests(unittest.TestCase):
    def dump(self, data, name):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'synthetic.db2'
            path.write_bytes(data)
            return oracle.dump_file(path, name, oracle.CATALOG_ID)

    def reject(self, data, name, code):
        with self.assertRaisesRegex(oracle.OracleError, '^' + code + '$'):
            self.dump(data, name)

    def test_opt_in_preserves_56_and_verifies_whole_artifact(self):
        schemas = oracle.load_catalog()
        self.assertEqual(len(schemas), 1126)
        self.assertEqual(len(oracle.SCHEMAS), 56)
        for name, schema in oracle.SCHEMAS.items():
            self.assertIs(oracle.resolve_schema(name, oracle.CATALOG_ID), schema)
        with self.assertRaisesRegex(oracle.OracleError, 'unsupported_table_schema'):
            oracle.resolve_schema('LiquidType')
        with self.assertRaisesRegex(oracle.OracleError, 'invalid_invocation'):
            oracle.resolve_schema('ChrClasses', 'other')
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'altered.json'
            path.write_bytes(catalog.FACTS_PATH.read_bytes() + b' ')
            with self.assertRaisesRegex(oracle.OracleError, 'unsupported_table_schema'):
                oracle.load_catalog(path=path)

    def test_extension_transcription_and_exact_empty_files(self):
        schemas = oracle.load_catalog()
        for fixture in CATALOG_EXTENSION_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                self.assertEqual(schemas[fixture.name], fixture)
                data = _zero_section_file(fixture, 0)
                self.assertEqual(self.dump(data, fixture.name), f'forever-db2-dump v1 {fixture.name}\n')
                self.reject(data[:-1], fixture.name, 'record_data_bounds')
                self.reject(data + b'\0', fixture.name, 'record_data_bounds')
                mismatch = bytearray(data)
                _put32(mismatch, 156, fixture.layout_hash ^ 1)
                self.reject(mismatch, fixture.name, 'unsupported_table_schema')

    def test_battle_pet_string_array_literal_golden_and_copy(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[0]
        data = Wdc5Builder(schema, [{0: ['a', '界', '', '\\x', '\t', 'ñ\n'], 1: 65535, 2: [0, 1, 127, 128, 254, 255]}], ids=[7], max_id=70000, copies=[(70000, 7)]).build()
        fields = '\tt:a,t:界,t:,t:\\\\x,t:\\t,t:ñ\\n\tu:65535\tu:0,u:1,u:127,u:128,u:254,u:255\n'
        self.assertEqual(self.dump(data, schema.name), 'forever-db2-dump v1 BattlePetEffectProperties\n7' + fields + '70000' + fields)

    def test_initiative_signed64_minimum_literal_golden_and_inline_copy(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[1]
        row = {0: -2**63, 1: 'é', 2: 'line\n', 3: 7, 4: -1, 5: -2**31, 6: 2**31-1, 7: 0, 8: 1}
        data = Wdc5Builder(schema, [row], max_id=70000, copies=[(70000, 7)]).build()
        self.assertEqual(self.dump(data, schema.name),
            'forever-db2-dump v1 InitiativeReward\n'
            '7\ti:-9223372036854775808\tt:é\tt:line\\n\tu:7\ti:-1\ti:-2147483648\ti:2147483647\ti:0\ti:1\n'
            '70000\ti:-9223372036854775808\tt:é\tt:line\\n\tu:70000\ti:-1\ti:-2147483648\ti:2147483647\ti:0\ti:1\n')

    def test_trait_signed64_maximum_and_signed_parent_copy_golden(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[3]
        row = {0: 2**63-1, 1: 7, 2: -9, 3: 255, 4: -2**31, 5: 2**31-1}
        data = Wdc5Builder(schema, [row], max_id=70000, copies=[(70000, 7)], parents=[(0xFFFFFFFF, 0)]).build()
        self.assertEqual(self.dump(data, schema.name),
            'forever-db2-dump v1 TraitCondAccountElement\n'
            '7\ti:9223372036854775807\tu:7\ti:-1\tu:255\ti:-2147483648\ti:2147483647\n'
            '70000\ti:9223372036854775807\tu:70000\ti:-1\tu:255\ti:-2147483648\ti:2147483647\n')

    def test_unsigned_byte_parent_is_full_u32_with_default_and_copies(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[4]
        row = {0: '界', 1: 7, 2: -2**31, 3: -1, 4: 2**31-1}
        for parent in [0, 255, 256, 0x80000000, 0xFFFFFFFF]:
            with self.subTest(parent=parent):
                data = Wdc5Builder(schema, [row], max_id=70000, copies=[(70000, 7)], parents=[(parent, 0)]).build()
                self.assertEqual(self.dump(data, schema.name),
                    'forever-db2-dump v1 UiWidgetVisTypeDataReq\n'
                    f'7\tt:界\tu:7\ti:-2147483648\ti:-1\ti:2147483647\tu:{parent}\n'
                    f'70000\tt:界\tu:70000\ti:-2147483648\ti:-1\ti:2147483647\tu:{parent}\n')
        data = Wdc5Builder(schema, [row]).build()
        self.assertEqual(self.dump(data, schema.name), 'forever-db2-dump v1 UiWidgetVisTypeDataReq\n7\tt:界\tu:7\ti:-2147483648\ti:-1\ti:2147483647\tu:0\n')
        invalid = Wdc5Builder(schema, [row], parents=[(0, 1)]).build()
        self.reject(invalid, schema.name, 'invalid_record_ids')

    def test_liquid_type_complete_literal_golden(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[2]
        bits = [0x80000000, 0x7FC12345, 0xFF800000, 1]
        row = {0: 'water', 1: ['', 'α', 'b\\c', 'line\n', 'tab\t', 'end'], 2: -1, 3: 255,
               4: 0xFFFFFFFF, 5: 7, 6: bits[0], 7: bits[1], 8: bits[2], 9: bits[3], 10: 65535,
               11: 0, 12: 1, 13: 2, 14: 3, 15: -2**31, 16: [0, 1, 2, 3, 4, 255],
               17: [-2**31, -1, 2**31-1], 18: bits * 9 + bits[:2], 19: [0, 1, 0x80000000, 0xFFFFFFFF], 20: bits}
        data = Wdc5Builder(schema, [row], ids=[7]).build()
        float_tokens = ['f:0x80000000', 'f:0x7fc12345', 'f:0xff800000', 'f:0x00000001']
        expected = ('forever-db2-dump v1 LiquidType\n7\tt:water\tt:,t:α,t:b\\\\c,t:line\\n,t:tab\\t,t:end'
                    '\ti:-1\tu:255\tu:4294967295\tu:7\tf:0x80000000\tf:0x7fc12345\tf:0xff800000\tf:0x00000001'
                    '\tu:65535\tf:0x00000000\tu:1\tu:2\tu:3\ti:-2147483648\tu:0,u:1,u:2,u:3,u:4,u:255'
                    '\ti:-2147483648,i:-1,i:2147483647\t' + ','.join(float_tokens * 9 + float_tokens[:2]) +
                    '\tu:0,u:1,u:2147483648,u:4294967295\t' + ','.join(float_tokens) + '\n')
        self.assertEqual(self.dump(data, schema.name), expected)

    def test_signed64_extremes_and_reject_every_compressed_mode(self):
        for schema in [CATALOG_EXTENSION_FIXTURE_SCHEMAS[1], CATALOG_EXTENSION_FIXTURE_SCHEMAS[3]]:
            for number in [-2**63, 2**63-1, -1, 0, 2**32+1]:
                data = Wdc5Builder(schema, [{0: number, schema.index: 7}]).build()
                self.assertIn(f'\n7\ti:{number}\t', self.dump(data, schema.name))
            for mode in range(1, 6):
                data = Wdc5Builder(schema, [{schema.index: 7}], modes={0: (mode, 0, 8, 1)}, common={0: (0, [(7, 0xFFFFFFFF)])}, pallet={0: [0xFFFFFFFF]}).build()
                self.reject(data, schema.name, 'invalid_column_data')

    def test_common64_rejection_also_covers_existing_unsigned_fields(self):
        schema = oracle.ITEM_EXTENDED_COST
        field = next(i for i, value in enumerate(schema.fields) if value.bits == 64)
        for entries in [[], [(7, 0xFFFFFFFF)]]:
            data = Wdc5Builder(schema, [{}], ids=[7], modes={field: (2, 0xFFFFFFFF, 0, 0)}, common={field: (0xFFFFFFFF, entries)}).build()
            self.reject(data, schema.name, 'invalid_column_data')

    def test_string_array_individual_invalid_offsets_utf8_and_termination(self):
        schema = CATALOG_EXTENSION_FIXTURE_SCHEMAS[0]
        base = Wdc5Builder(schema, [{0: ['one', 'two', 'three', 'four', 'five', 'six']}], ids=[7]).build()
        start = int.from_bytes(base[212:216], 'little')
        for array in range(6):
            invalid = bytearray(base)
            _put32(invalid, start + 4 * array, 0xFFFFFFFF)
            self.reject(invalid, schema.name, 'invalid_record_string')
        strings_start = start + int.from_bytes(base[144:148], 'little')
        strings_end = strings_start + int.from_bytes(base[148:152], 'little')
        invalid = bytearray(base)
        invalid[strings_start] = 0xFF
        self.reject(invalid, schema.name, 'invalid_record_string')
        invalid = bytearray(base)
        invalid[strings_end - 1] = ord('x')
        self.reject(invalid, schema.name, 'invalid_record_string')
        invalid = bytearray(base)
        _put32(invalid, 244 + 3 * 4 + 8, 1)
        self.reject(invalid, schema.name, 'invalid_column_data')
        _put32(invalid, 136, 80000)
        self.reject(invalid, schema.name, 'record_policy_limit')


if __name__ == '__main__':
    unittest.main()
