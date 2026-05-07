import os
import subprocess
import sys


def run(cmd):
    p = subprocess.run([sys.executable, 'epictracker.py'] + cmd, capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def test_scan():
    rc, out, err = run(['scan'])
    assert rc == 0
    assert 'Arcs:' in out


def test_show_player():
    rc, out, err = run(['show-player', 'sample_player'])
    assert rc == 0
    assert 'sample_player/SampleChar' in out
