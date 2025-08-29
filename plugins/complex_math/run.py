#!/usr/bin/env python3
"""Bootstrap a virtual environment and run the complex_math server."""
import os
import subprocess
import sys

HERE = os.path.abspath(os.path.dirname(__file__))
VENV_DIR = os.path.join(HERE, '.venv')

if not os.path.exists(VENV_DIR):
    subprocess.check_call([sys.executable, '-m', 'venv', VENV_DIR])
    if os.name == 'nt':
        pip_cmd = os.path.join(VENV_DIR, 'Scripts', 'pip.exe')
    else:
        pip_cmd = os.path.join(VENV_DIR, 'bin', 'pip')
    subprocess.check_call(
        [pip_cmd, 'install', '--upgrade', 'pip'],
        stdout=sys.stderr,
        stderr=sys.stderr,
    )
    subprocess.check_call(
        [pip_cmd, 'install', '-r', os.path.join(HERE, 'requirements.txt')],
        stdout=sys.stderr,
        stderr=sys.stderr,
    )

if os.name == 'nt':
    python = os.path.join(VENV_DIR, 'Scripts', 'python.exe')
else:
    python = os.path.join(VENV_DIR, 'bin', 'python')

os.execv(python, [python, os.path.join(HERE, 'server.py')])
