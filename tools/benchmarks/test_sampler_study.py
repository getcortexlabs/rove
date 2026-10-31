import json
from pathlib import Path
import tempfile
import unittest

from sampler_study import publish, sample_digest, validate_run


class StudyTests(unittest.TestCase):
    def test_sample_hash_ignores_provenance_but_not_tree_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp) / 'a', Path(tmp) / 'b'
            a.write_text('#NEXUS\n[command first]\ntree STATE_0 = (A:1,B:1);\nEnd;\n')
            b.write_text('#NEXUS\n[command second]\ntree STATE_0 = (A:1,B:1);\nEnd;\n')
            self.assertEqual(sample_digest(a, True), sample_digest(b, True))
            b.write_text('tree STATE_0 = (A:2,B:1);\n')
            self.assertNotEqual(sample_digest(a, True), sample_digest(b, True))

    def test_zero_exit_with_truncated_output_is_not_a_completed_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            record = {'status': 'completed', 'completed_steps': 2,
                'input': {'sha256': 'hash', 'gaps': 'missing'},
                'settings': {'seed': 1, 'sample_every': 1, 'tuning': True},
                'model': {'conditioning': 'none (Mk)', 'monophyly': [],
                          'sample_from_prior': False, 'partitioning': 'observed'}}
            (out / 'run.json').write_text(json.dumps(record))
            (out / 'trace.log').write_text('Sample\tposterior\n0\t-1\n1\t-1\n')
            metadata = {'inputs': {'morphology90': {'sha256': 'hash'}}}
            with self.assertRaisesRegex(ValueError, 'Truncated trace'):
                validate_run(out, metadata, 'morphology90', 2, 1, 1)
            (out / 'trace.log').write_text('Sample\tposterior\n0\t-1\n1\t-1\n2\t-1\n')
            (out / 'trees.nexus').write_text('tree STATE_0 = (A:1,B:1);\nEnd;\n')
            with self.assertRaisesRegex(ValueError, 'Truncated tree'):
                validate_run(out, metadata, 'morphology90', 2, 1, 1)

    def test_export_does_not_read_private_raw_records(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            metadata = {'inputs': {'zilla': {'path': '/private/alignment.nex', 'sha256': 'hash'}}}
            (out / 'metadata.json').write_text(json.dumps(metadata))
            (out / 'profiles.json').write_text(json.dumps([{'command': ['/private/alignment.nex'], 'count': 5}]))
            (out / 'comparison.json').write_text('{"taxa": ["PRIVATE_TAXON"]}')
            publish(out, out / 'public', 'profile')
            text = (out / 'public/results.json').read_text()
            self.assertNotIn('PRIVATE_TAXON', text)
            self.assertNotIn('/private/alignment.nex', text)
            self.assertIn('$PRIVATE_MATRIX', text)


if __name__ == '__main__': unittest.main()
