import json
import os
import sys

arguments = sys.argv[1:]
if "--bundle" in arguments:
    bundle = arguments[arguments.index("--bundle") + 1]
    path = os.path.join(bundle, "config.json")
    with open(path, encoding="utf-8") as handle:
        spec = json.load(handle)
    memory = spec.get("linux", {}).get("resources", {}).get("memory", {})
    if memory.pop("disableOOMKiller", None) is not None:
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(spec, handle)
os.execv("/usr/bin/crun", ["crun", *arguments])
