# SPDX-License-Identifier: GPL-3.0-or-later
"""Selected-map Linux plan surface guards; executable evidence is separate."""
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def compact(source):
    return re.sub(r'\s+', '', source).replace(',)', ')')


class LinuxMapDomainContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads((ROOT / 'contracts/forever-linux-map-domain-contract-v1.json').read_text())
        cls.source = {key: (ROOT / path).read_text() for key, path in cls.contract['sources'].items()}

    def test_v8_keeps_reserved_v7_and_previous_formats(self):
        c = self.contract
        self.assertEqual(c['product_plan_version'], 8)
        self.assertEqual(c['reserved_product_plan_versions'], [7])
        self.assertEqual(c['legacy_product_plan_versions_unchanged'], list(range(1, 7)))
        self.assertEqual(c['package_manifest_version_unchanged'], 1)
        self.assertEqual(c['package_state_version_unchanged'], 2)
        self.assertEqual(c['new_dependencies'], [])
        self.assertIn('"8" => 8', self.source['plan'])
        self.assertNotIn('"7" => 7', self.source['plan'])

    def test_explicit_domain_and_global_startup_policies(self):
        c = self.contract
        self.assertEqual(c['platform'], 'linux')
        self.assertEqual(c['builds'], [70170, 70205])
        self.assertTrue(c['global_gameobjects_retained'])
        self.assertTrue(all(value is False for value in c['claims'].values()))
        for name, value in [('MAP_DOMAIN_PROFILE', c['map_domain_profile']),
                            ('GAMEOBJECT_SCOPE_PROFILE', c['gameobject_scope']),
                            ('PLACEMENT_ID_DOMAIN_PROFILE', c['placement_id_domain']),
                            ('MAP_DOMAIN_VERIFICATION_PROFILE', c['verification_profile'])]:
            self.assertIn(f'{name}: &str = "{value}"', self.source['domain'])
        self.assertIn(c['verification_scope'], self.source['domain'])

    def test_plan_version_and_contract_precede_legacy_inference(self):
        plan = self.source['plan']
        for method, marker in [('version', 'return 8'), ('contract', 'return "forever-product-v8"')]:
            body = plan.split(f'pub const fn {method}(', 1)[1].split('pub const fn', 1)[0]
            self.assertLess(body.index('self.map_domain.is_some()'), body.index('self.offmesh.is_some()'))
            self.assertIn(marker, body)
        self.assertIn('pub map_domain: Option<u32>', plan)

    def test_canonical_domain_fields_are_after_disk_before_components(self):
        plan = self.source['plan']
        domain = self.source['domain']
        self.assertIn('{collision_storage}{map_domain}components', plan)
        for key in self.contract['canonical_fields_after_collision_storage']:
            self.assertIn(f'field(lines, "{key}")?', domain)
        self.assertLess(domain.index('field(lines, "map-domain")'), domain.index('field(lines, "map-id")'))
        self.assertLess(domain.index('field(lines, "map-id")'), domain.index('field(lines, "gameobject-scope")'))
        self.assertLess(domain.index('field(lines, "gameobject-scope")'), domain.index('field(lines, "placement-ids")'))

    def test_new_scope_requires_existing_disk_and_geometry_sources(self):
        domain = self.source['domain']
        for marker in ['SchemaCatalog::PinnedMetadataV1', 'CollisionStorage::LeasedDiskV1',
                       'Component::Db2, Component::Terrain, Component::Collision',
                       'entry.name == "Map"', 'plan.terrain != Selection::Map(id)',
                       'map != id', 'connection.map_id != id']:
            self.assertIn(marker, domain)
        self.assertIn('self.map_domain.is_none() && self.terrain != Selection::All', self.source['plan'])
        self.assertIn('!has_db2("LiquidType")', self.source['plan'])

    def test_v8_navigation_offmesh_presence_is_explicit(self):
        self.assertIn('version == 8 && navigation != Selection::None', self.source['plan'])
        self.assertIn('(plan.navigation != Selection::None) != plan.offmesh.is_some()', self.source['domain'])
        self.assertIn('product_plan_map_domain_offmesh', self.source['domain'])

    def test_cli_reports_declared_domain_without_invented_dependency_inventory(self):
        main = self.source['cli']
        for key in ['map_domain', 'requested_map', 'gameobject_scope', 'placement_ids']:
            self.assertIn(key, main)
        self.assertIn('plan.map_domain.map_or_else', main)
        self.assertIn('MAP_DOMAIN_VERIFY_SCOPE', main)
        self.assertIn('MAP_DOMAIN_VERIFICATION_PROFILE', main)
        self.assertIn('full_extractor_complete\\\":false', main)
        self.assertNotIn('dependency_map_ids', main)

    def test_exact_plan_unit_inventory_and_legacy_hash_gates(self):
        names = re.findall(r'#\[test\]\s*fn (\w+)\(', self.source['plan_tests'])
        self.assertEqual(names, self.contract['plan_test_names'])
        self.assertEqual(len(names), 16)
        self.assertIn('legacy_v1_to_v6_exact_render_bytes_and_fingerprints_are_preserved', names)
        self.assertNotIn('#[ignore', self.source['plan_tests'])

    def test_complete_map_preflight_precedes_output_create_and_resume(self):
        driver = self.source['driver']
        body = driver.split('fn extract_inner(', 1)[1].split('fn prepare_domain(', 1)[0]
        marker = 'let domain = prepare_domain(plan, &mut store)?;'
        self.assertLess(body.index(marker), body.index('Run::resume('))
        self.assertLess(body.index(marker), body.index('Run::create('))
        prepare = driver.split('fn prepare_domain(', 1)[1].split('#[cfg(test)]', 1)[0]
        self.assertIn('read(store, terrain::run::MAP_ID, DB2_FILE_BYTES)?', prepare)
        self.assertLess(prepare.index('if peak > 2 * 1024 * 1024 * 1024'),
                        prepare.index('Domain::derive('))
        self.assertIn('store.stats().resident_bytes', prepare)
        self.assertIn('let Some(requested) = plan.map_domain else', prepare)
        self.assertIn('return Ok(None);', prepare)

    def test_all_domain_projections_and_digest_use_the_same_buffer(self):
        derived = self.source['derived_domain']
        for marker in ('source::read_vmap_maps(bytes)', 'source::read_map(bytes)',
                       'source::read_navigation_maps(bytes)', 'integrity::digest(bytes)',
                       'row.fields.wdt_file_data_id as u32 != entry.wdt',
                       'row.fields.effective_parent()', 'parent != entry.parent'):
            self.assertIn(marker, derived)
        self.assertLess(derived.index('drop(rows);'), derived.index('source::read_map(bytes)'))
        self.assertLess(derived.index('drop(terrain);'), derived.index('source::read_navigation_maps(bytes)'))
        for forbidden in ('std::fs', 'File::open', 'LocalStore::open', 'unsafe {'):
            self.assertNotIn(forbidden, derived)
        self.assertIn('domain.verify_source(&bytes)?;', self.source['driver'])
        self.assertIn('domain.verify_source(&map)?;', self.source['driver'])

    def test_domain_and_verifier_metadata_ceilings_remain_bounded(self):
        limits = self.contract['limits']
        self.assertEqual(limits['selected_maps'], 1024)
        self.assertEqual(limits['core_selection_logical_bytes'], 64 * 1024)
        self.assertEqual(limits['product_compact_domain_logical_bytes'], 64 * 1024)
        self.assertEqual(limits['verifier_map_fact_vectors'], 5)
        self.assertEqual(limits['verifier_fact_logical_bytes'], 24 * 1024)
        self.assertEqual(limits['product_domain_plus_navigation_rows_logical_bytes'], 64 * 1024)
        self.assertFalse(limits['exact_rss_claim'])
        self.assertFalse(limits['all_cpu_work_claim'])
        self.assertFalse(limits['existing_component_ceilings_raised'])
        self.assertIn('MAX_DOMAIN_MAPS: usize = 1024', self.source['core_plan'])
        self.assertIn('MAX_DOMAIN_BYTES: usize = 64 * 1024', self.source['core_plan'])
        self.assertIn('count > limits.max_domain_maps', self.source['core_plan'])
        self.assertIn('bytes > limits.max_domain_bytes', self.source['core_plan'])
        self.assertIn('MAX_DOMAIN_BYTES: usize = 64 * 1024', self.source['derived_domain'])
        self.assertIn('compact_bytes(entries.capacity(), ids.capacity())? > MAX_DOMAIN_BYTES', self.source['derived_domain'])
        verifier = self.source['verify']
        for field in ('domain_terrain_maps', 'domain_terrain_lists',
                      'domain_collision_maps', 'domain_collision_payload_maps',
                      'domain_navigation_globals'):
            self.assertIn(f'{field}: Vec<u32>', verifier)
        allocation = verifier.split('fn domain_map(', 1)[1].split('fn verify_offmesh_sources(', 1)[0]
        self.assertLess(allocation.index('maps.len() >= 1024'), allocation.index('.try_reserve_exact(1)'))
        self.assertIn('maps.capacity() > 1024', allocation)

    def test_selected_core_preserves_order_and_complete_topology(self):
        core = compact(self.source['core_plan'])
        self.assertIn('Self::from_materialized_selection(rows,direct_count,MapSelection::All,limits)', core)
        self.assertIn('preload.sort_unstable()', core)
        self.assertIn('processing.sort_unstable_by_key(|&ordinal|(nodes[ordinal].depth,ordinal))', core)
        self.assertIn('ordinal>=direct_count', core)
        self.assertIn('rows[ordinal].fields.wdt_file_data_id==0', core)
        self.assertIn('current=nodes[ordinal].parent', core)
        self.assertIn('selected_active', self.source['core_world'])
        self.assertIn('self.selected.binary_search(&entry.physical_ordinal)', self.source['core_world'])
        self.assertIn('maps.all().len()', self.source['core_world_input'])
        self.assertIn('maps.domain_bytes()', self.source['core_world_input'])
        staged = self.source['core_staged']
        self.assertIn('world::inventory::extract_selected(', staged)
        self.assertIn('topology.selected_active().count()', staged)
        self.assertIn('topology.contains_selected(m.id())', staged)
        self.assertIn('world::inventory::', staged)
        self.assertIn('from_materialized_selection(', self.source['core_inventory'])
        self.assertIn('topology().contains_selected(id)', self.source['core_package'])

    def test_selected_consumers_reduce_existing_budgets(self):
        driver = self.source['driver']
        self.assertIn('terrain::plan::Selection::MapIds(domain.ids())', driver)
        self.assertEqual(driver.count('.checked_sub(domain.resident_bytes())'), 1)
        self.assertEqual(driver.count('.checked_sub(caller_bytes)'), 2)
        self.assertIn('limits.max_memory_bytes', driver)
        self.assertIn('discovery_limits.heap_bytes', driver)
        self.assertIn('run_limits.pipeline.heap_bytes', driver)
        disk = compact(self.source['disk_driver'])
        self.assertIn('COMBINED.checked_sub(domain.map_or(0,|domain|domain.resident_bytes()))', disk)
        self.assertIn('startup_limits.collision.max_memory_bytes.min(combined)', disk)
        self.assertIn('stage_limits.world.max_combined_memory_bytes=combined', disk)
        self.assertIn('model_limits.max_memory_bytes=combined', disk)
        self.assertIn('package_limits.max_memory_bytes=combined', disk)
        self.assertIn('gameobjects::spooled::extract_startup_fdid(', disk)
        self.assertIn('staged::extract_selected(', disk)
        self.assertIn('None=>staged::extract(', disk)

    def test_v8_verification_enforces_exact_map_domains_and_links(self):
        verifier = compact(self.source['verify'])
        self.assertIn('(!plan.is_v12()&&(matches!(scope,Scope::Links)||plan.map_domain.is_some())).then(links::Facts::default)', verifier)
        self.assertIn('ifplan.map_domain.is_none(){returnOk(());}', verifier)
        self.assertIn('domain.ids().len()!=self.domain_terrain_lists.len()', verifier)
        self.assertIn('!self.domain_terrain_lists.contains(id)', verifier)
        self.assertIn('self.trees.len()!=domain.ids().len()', verifier)
        self.assertIn('!self.trees.contains_key(id)', verifier)
        self.assertIn('self.domain_terrain_maps.contains(id)||self.domain_collision_payload_maps.contains(id)', verifier)
        self.assertIn('domain.ids().iter().filter(|id|own_navigation(id))', verifier)
        self.assertIn('self.domain_navigation_globals.len()!=expected_globals.clone().count()', verifier)
        self.assertIn('expected_globals.any(|id|!self.domain_navigation_globals.contains(id))', verifier)
        self.assertIn('Domain::derive(bytes,requested,plan.contains(Component::Navigation))', verifier)
        self.assertLess(verifier.index('validate_db2(plan,name,bytes)?'), verifier.index('Domain::derive('))
        self.assertLess(verifier.index('product_plan_mismatch'), verifier.index('facts.finish_domain(plan)?'))
        self.assertLess(verifier.index('facts.finish_domain(plan)?'), verifier.index('links.finish(plan)?'))
        self.assertIn('ifplan.map_domain.is_none(){admit(plan.navigation,id,None,true)?;}', verifier)
        self.assertIn('ifplan.map_domain.is_none(){admit(plan.navigation,map,Some((col,row)),true)?;}', verifier)

    def test_navigation_projection_is_closed_direct_numeric_and_budgeted(self):
        derived = compact(self.source['derived_domain'])
        projection = derived.split('fnnavigation_rows(', 1)[1].split('fnnavigation_resident_bytes(', 1)[0]
        self.assertIn('forrowinsource.direct_records()', projection)
        self.assertIn('if!self.contains(row.id){continue;}', projection)
        self.assertIn('rows.iter().any(|existing|existing.id==row.id)', projection)
        self.assertIn('entry.parent!=parent', projection)
        self.assertIn('rows.push(row.clone())', projection)
        self.assertIn('rows.len()!=self.ids.len()', projection)
        self.assertNotIn('materialize(', projection)
        self.assertNotIn('SourceRecords{', derived)
        self.assertLess(projection.index('requested>MAX_DOMAIN_BYTES'), projection.index('rows.try_reserve_exact('))
        self.assertLess(projection.index('self.navigation_resident_bytes(rows.capacity())?'), projection.index('forrowinsource.direct_records()'))
        resident = derived.split('fnnavigation_resident_bytes(', 1)[1].split('fnverify_source(', 1)[0]
        self.assertIn('capacity.checked_mul(size_of::<source::DirectRecord<source::NavigationMapFields>>())', resident)
        self.assertIn('.checked_add(self.resident_bytes())', resident)
        self.assertIn('bytes>MAX_DOMAIN_BYTES', resident)
        driver = compact(self.source['driver'])
        self.assertLess(driver.index('letrows=domain.navigation_rows(&source)?'), driver.index('drop(source);'))
        self.assertLess(driver.index('drop(source);'), driver.index('navigation_input::Package::open_rows('))
        self.assertIn('domain.navigation_resident_bytes(rows.capacity())?', driver)
        self.assertIn('navigation_input::Package::open_rows(&output.join("maps"),&output.join("vmaps"),rows,plan.build,inventory::Options::default(),discovery_limits)', driver)
        self.assertIn('input.plan().maps().iter().any(|entry|!domain.contains(entry.map_id()))', driver)
        self.assertIn('input.plan().maps().len()!=domain.ids().len()', driver)
        self.assertIn('letselection=ifdomain.is_some(){nav::Selection::All}else{selection}', driver)

    def test_navigation_dependency_and_empty_omission_policy_is_explicit(self):
        policy = self.contract['navigation_output_policy']
        self.assertEqual(policy['profile'], 'selected-domain-own-candidates-v1')
        self.assertIn('ancestor members', policy['ancestor_outputs'])
        self.assertIn('no global or tile', policy['own_empty_map'])
        self.assertIn('zero navigation files', policy['wholly_empty_domain'])
        self.assertIn('.vmtileidx is excluded', policy['candidate_union'])
        self.assertIn('requested map only', self.contract['offmesh_map_scope'])
        self.assertIn('maps without own candidates emit no MMaps', self.source['cli'])
        verifier = self.source['verify']
        payload_branch = verifier.split('if file.ends_with(".vmtileidx")', 1)[1].split('Component::Navigation =>', 1)[0]
        self.assertNotIn('domain_collision_payload_maps', payload_branch.split('} else {', 1)[0])
        payload = payload_branch.split('} else {', 1)[1]
        self.assertLess(payload.index('assembly::TileView::parse('), payload.index('domain_collision_payload_maps'))

    def test_corresponding_source_selects_new_contract_and_plan_files(self):
        spec = importlib.util.spec_from_file_location('map_domain_packager', ROOT / 'tools/package_forever.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        selected = set(module.selected_paths(ROOT))
        for path in [*self.contract['sources'].values(),
                     'contracts/forever-linux-map-domain-contract-v1.json',
                     'tests/contracts/test_forever_linux_map_domain_contract.py']:
            self.assertIn(path, selected)
            self.assertTrue(module.bundle_path_allowed('source/' + path))


if __name__ == '__main__':
    unittest.main()
