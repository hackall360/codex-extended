#!/usr/bin/env python3
"""Create a local Python virtual environment and install requirements."""
import os
import subprocess
import sys

VENV_DIR = os.path.join(os.path.dirname(__file__), '..', '.venv')
VENV_DIR = os.path.abspath(VENV_DIR)

if not os.path.exists(VENV_DIR):
    subprocess.check_call([sys.executable, '-m', 'venv', VENV_DIR])

if os.name == 'nt':
    pip_executable = os.path.join(VENV_DIR, 'Scripts', 'pip.exe')
else:
    pip_executable = os.path.join(VENV_DIR, 'bin', 'pip')

subprocess.check_call([pip_executable, 'install', '--upgrade', 'pip'])
subprocess.check_call([pip_executable, 'install', '-r', os.path.join(os.path.dirname(__file__), '..', 'requirements.txt')])
