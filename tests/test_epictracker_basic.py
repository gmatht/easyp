import subprocess
import sys
import unittest


def run(cmd):
    p = subprocess.run([sys.executable, 'epictracker.py'] + cmd, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


class EpicTrackerCLITest(unittest.TestCase):
    def test_scan(self):
        rc, out, err = run(['scan'])
        self.assertEqual(rc, 0)
        self.assertIn('Arcs:', out)

    def test_show_player(self):
        rc, out, err = run(['show-player', 'sample_player'])
        self.assertEqual(rc, 0)
        self.assertIn('sample_player/SampleChar', out)


if __name__ == '__main__':
    unittest.main()
