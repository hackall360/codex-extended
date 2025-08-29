#!/usr/bin/env python3
import argparse
import subprocess
import sys
import re

LEVEL_THRESHOLDS = {
    "feature": 0.60,
    "develop": 0.70,
    "debug": 0.85,
    "release": 0.95,
}

parser = argparse.ArgumentParser(description="Run cargo tests and enforce pass rate thresholds")
parser.add_argument("--level", choices=LEVEL_THRESHOLDS.keys(), default="feature", help="testing level")
parser.add_argument("--target-pattern", help="optional test name pattern for targeted features")
args, extra = parser.parse_known_args()

base_cmd = ["cargo", "test", "--manifest-path", "omni-bridge/Cargo.toml", "--features", "openai anthropic"]
# run full test suite
proc = subprocess.run(base_cmd + extra, capture_output=True, text=True)
output = proc.stdout + proc.stderr
print(output)
matches = re.findall(r"test result: (ok|FAILED). (\d+) passed; (\d+) failed; (\d+) ignored", output)
if not matches:
    print("Could not parse test output")
    sys.exit(1)
passed = sum(int(m[1]) for m in matches)
failed = sum(int(m[2]) for m in matches)
total = passed + failed
rate = passed / total if total else 1.0
threshold = LEVEL_THRESHOLDS[args.level]

# targeted feature testing if requested
if args.target_pattern:
    tgt_proc = subprocess.run(base_cmd + [args.target_pattern], capture_output=True, text=True)
    tgt_out = tgt_proc.stdout + tgt_proc.stderr
    print(tgt_out)
    tgt_matches = re.findall(r"test result: (ok|FAILED). (\d+) passed; (\d+) failed; (\d+) ignored", tgt_out)
    if not tgt_matches:
        print("Could not parse target test output")
        sys.exit(1)
    tgt_passed = sum(int(m[1]) for m in tgt_matches)
    tgt_failed = sum(int(m[2]) for m in tgt_matches)
    tgt_total = tgt_passed + tgt_failed
    tgt_rate = tgt_passed / tgt_total if tgt_total else 1.0
    print(f"Target pass rate: {tgt_passed}/{tgt_total} = {tgt_rate:.2%}")
    if args.level == "feature" and tgt_rate < 0.80:
        print("Target feature pass rate below 80% threshold")
        sys.exit(1)

print(f"Overall pass rate: {passed}/{total} = {rate:.2%}")
if rate >= threshold:
    print(f"{args.level} threshold {threshold:.0%} met")
    sys.exit(0)
else:
    print(f"{args.level} threshold {threshold:.0%} NOT met")
    sys.exit(1)
