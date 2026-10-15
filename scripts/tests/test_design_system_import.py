import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

SPEC = importlib.util.spec_from_file_location('design_check', Path(__file__).parents[1] / 'check-design-system.py')
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


class ImportIntegrityTests(unittest.TestCase):
    def test_archive_tampering_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            imported = root / CHECK.IMPORT
            imported.mkdir(parents=True)
            (imported / 'source.zip').write_bytes(b'changed archive')
            (imported / 'import.json').write_text(json.dumps({'archive_sha256': CHECK.ARCHIVE_SHA}))
            self.assertIn('original archive hash changed', CHECK.archive_errors(root))

    def test_replacing_manifest_and_goldens_together_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            folder = root / 'design/baseline'
            folder.mkdir(parents=True)
            (folder / 'changed.png').write_bytes(b'changed')
            (folder / 'manifest.json').write_text(json.dumps({'snapshots': {'files': [{'file': 'changed.png', 'sha256': CHECK.sha(b'changed')}]}}))
            self.assertIn('historical manifest changed', CHECK.historical_errors(root)[0])

    def test_missing_and_outside_references_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            origin = root / 'system.json'
            origin.write_text('{}')
            for ref in ('missing.json', '../outside.json', '#/missing'):
                self.assertTrue(CHECK.references_errors({'$ref': ref}, origin, root))

    def test_local_pointer_is_resolved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            origin = root / 'system.json'
            origin.write_text('{"tokens": {"a/b": [1]}}')
            self.assertFalse(CHECK.references_errors({'$ref': '#/tokens/a~1b/0'}, origin, root))


if __name__ == '__main__':
    unittest.main()
