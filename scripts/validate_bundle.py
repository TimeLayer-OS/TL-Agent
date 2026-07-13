#!/usr/bin/env python3
"""Validate a TL-Agent bundle against schemas/ before running it.
Usage: python3 scripts/validate_bundle.py <bundle_dir>
Exit 0 = valid, 1 = invalid (fail-closed). Requires: pip install jsonschema
"""
import json, sys, pathlib
try:
    import jsonschema
except ImportError:
    sys.exit("need jsonschema: pip install jsonschema")

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCH = ROOT / "schemas"
def load(p): return json.loads(pathlib.Path(p).read_text(encoding="utf-8"))

def check(inst_path, schema_name):
    jsonschema.validate(load(inst_path), load(SCH/schema_name))
    print("OK", inst_path)

def main():
    b = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "example-bundle")
    check(b/"manifest.json", "manifest-v1.json")
    topo = b/"topology.json"
    if not topo.exists(): topo = b/"topology_segment.json"
    check(topo, "topology-v1.json")
    for pol, sch in [("policies/agent_policy.json","agent-policy-v1.json"),
                     ("policies/stop_policy.json","stop-policy-v1.json")]:
        if (b/pol).exists(): check(b/pol, sch)
    manifest = load(b/"manifest.json")
    for aid in manifest["actions"]:
        env = b/"receipts"/aid/"envelope.json"
        if env.exists(): check(env, "envelope-v1.json")
    print("BUNDLE VALID")

if __name__ == "__main__":
    try:
        main()
    except jsonschema.ValidationError as e:
        print("INVALID:", e.message); sys.exit(1)
