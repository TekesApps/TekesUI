#!/usr/bin/env python3
"""Read current capability context; historical proposal tools remain separate."""
from pathlib import Path
import runpy
import sys

if __name__ == '__main__':
    sys.argv[1:1] = ['--context']
    runpy.run_path(str(Path(__file__).with_name('generate-component-capabilities.py')), run_name='__main__')
