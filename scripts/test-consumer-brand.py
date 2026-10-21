#!/usr/bin/env python3
"""Two external caller workspaces, exact WIP dependencies, distinct own BrandConfig.

CI uses CPU scene validation. Set TEKESUI_BRAND_GPU_EVIDENCE to an empty output
folder for the same public consumer scenes rendered on a real GPU, without UI.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from tekesui_build import build_lock, environment, prepare_cache
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('consumer_brand', ROOT / 'scripts/compile-consumer-brand.py')
compiler = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compiler)


class ConsumerBrandTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='tekesui-external-brands-')
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)

    def config(self, name, seed):
        path = self.directory / (name + '.json')
        path.write_text(json.dumps(dict(schema_version='1.0', brand_id=name, version='1.0.0', seed_srgb=seed, overrides={})))
        return path

    def test_config_errors_and_output_collision_leave_checkout_and_destination_untouched(self):
        before = (ROOT / 'design/brand/config.json').read_bytes()
        for config in [dict(schema_version='99', brand_id='bad', version='1.0.0', seed_srgb='#123456', overrides={}),
                       dict(schema_version='1.0', brand_id='bad', version='1.0.0', seed_srgb='blue', overrides={}),
                       dict(schema_version='1.0', brand_id='bad', version='1.0.0', seed_srgb='#123456', overrides={'unknown-role':{}})]:
            path = self.directory / 'invalid.json'; path.write_text(json.dumps(config))
            output = self.directory / 'invalid-artifact'
            with self.assertRaises(ValueError): compiler.compile_artifact(path, output)
            self.assertFalse(output.exists())
        output = self.directory / 'occupied'; output.mkdir(); marker = output / 'owned'; marker.write_text('retain')
        with self.assertRaises(ValueError): compiler.compile_artifact(self.config('a', '#B57251'), output)
        self.assertEqual(marker.read_text(), 'retain')
        broken = self.directory / 'broken-link'; broken.symlink_to(self.directory / 'absent')
        with self.assertRaises(ValueError): compiler.compile_artifact(self.config('b', '#367F7B'), broken)
        self.assertTrue(broken.is_symlink())
        with self.assertRaisesRegex(ValueError, 'outside'):
            compiler.compile_artifact(self.config('c', '#367F7B'), ROOT / ('must-not-create-artifact-' + str(os.getpid())))
        self.assertEqual((ROOT / 'design/brand/config.json').read_bytes(), before)

    @unittest.skipUnless(sys.platform == 'darwin', 'CoreText public consumer requires macOS')
    @build_lock()
    def test_two_external_consumers_and_real_control_scenes(self):
        prepare_cache()
        evidence = os.environ.get('TEKESUI_BRAND_GPU_EVIDENCE')
        output_root = Path(evidence).resolve() if evidence else self.directory / 'evidence'
        output_root.mkdir(parents=True, exist_ok=False)
        before = {p:hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in compiler.merge.SOURCE_FILES}
        results = []
        documents = []
        for name, seed in [('consumer-apricot','#B57251'),('consumer-seaglass','#367F7B')]:
            project = self.directory / name
            shutil.copytree(ROOT / 'examples/brand-consumer', project, ignore=shutil.ignore_patterns('target'))
            manifest = project / 'Cargo.toml'; manifest.write_text(manifest.read_text().replace('../../crates/',str(ROOT / 'crates') + '/'))
            own_config = self.config(name,seed); shutil.copy2(own_config, project / 'BrandConfig.json')
            artifact = compiler.compile_artifact(project / 'BrandConfig.json', project / 'offline-design')
            env = dict(environment(), TEKESUI_DESIGN_ARTIFACT=str(artifact))
            output = output_root / name
            command = ['cargo','run','--locked','--offline','--manifest-path',str(manifest),'--'] + ([] if evidence else ['--check']) + [str(output)]
            run = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            (output_root / (name + '-build.log')).write_text(run.stdout)
            self.assertEqual(run.returncode,0,run.stdout[-5000:])
            result = json.loads((output / 'result.json').read_text()); self.assertEqual(result['brand'],name);self.assertEqual(result['seed'],seed)
            documents.append(json.loads((artifact / "design/tokens.json").read_text()))
            results.append(result)
            shutil.copy2(project / 'BrandConfig.json', output / 'BrandConfig.json')
            (output / 'artifact-manifest.json').write_text(json.dumps(json.loads((artifact / 'design/tokens.json').read_text())['merge'],indent=2))
        # The same real Cargo integration must fail when an explicit artifact
        # disappears; a previously compiled valid brand must not mask the error.
        env['TEKESUI_DESIGN_ARTIFACT'] = str(self.directory / 'missing-artifact')
        rejected = subprocess.run(['cargo','check','--locked','--offline','--manifest-path',str(manifest)],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (output_root / 'missing-artifact-build.log').write_text(rejected.stdout)
        self.assertNotEqual(rejected.returncode,0)
        self.assertIn('must be available',rejected.stdout)
        fixed = ['primary-text','secondary-text','border-focused','text-selection','row-selection','status-failed','diff-added-surface','separator']
        for a,b in zip(results[0]['themes'],results[1]['themes']):
            self.assertEqual(a['theme'],b['theme'])
            self.assertNotEqual(a['roles']['action-primary-ground'],b['roles']['action-primary-ground'])
            for role in fixed:self.assertEqual(a['roles'][role],b['roles'][role],role)
            for result in [a,b]:
                states = [result['roles'][r] for r in ['action-primary-ground','action-primary-hover','action-primary-pressed']]
                self.assertEqual(len({tuple(c) for c in states}),3)
        baseline = json.loads((ROOT / 'design/tokens.json').read_text())['proposal']['tokens']
        affected = sys.modules['brand'].dependency_names(baseline)
        independent = {item['name'] for item in baseline['color']['tokens']} - affected
        for document in documents:
            tokens = document['proposal']['tokens']
            for role in independent:
                for theme in compiler.merge.THEMES:
                    self.assertEqual(compiler.merge.token_value(tokens,'color',role,theme),compiler.merge.token_value(baseline,'color',role,theme),role)
        after = {p:hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in before}
        self.assertEqual(before,after)
        (output_root / 'receipt.json').write_text(json.dumps({'source':'exact dependency checkout plus all WIP; no fork or edits','dependency_root':str(ROOT),'source_sha256':before,'consumers':[r['brand'] for r in results],'four_themes':True,'state_and_semantic_isolation':True,'actual_gpu':bool(evidence),'native':False,'static_independent_color_roles':len(independent),'runtime_roles_per_theme':len(results[0]['themes'][0]['roles'])},indent=2))


if __name__ == '__main__': unittest.main()
