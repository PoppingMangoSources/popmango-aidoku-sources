#!/usr/bin/env bash
# Bumps a source's version and the two .aix links on its README row, and nothing else.
# Every push to main that changes a source publishes it, so every such push bumps.
#
# Usage: scripts/bump.sh <id>

set -euo pipefail

if [ $# -ne 1 ]; then
	sed -n '5p' "$0" | sed 's/^# //'
	exit 1
fi

root=$(git rev-parse --show-toplevel)
python3 - "$root" "$1" <<'EOF'
import json, re, sys
from pathlib import Path

root, source_id = Path(sys.argv[1]), sys.argv[2]
manifest = root / "sources" / source_id / "res" / "source.json"
info = json.loads(manifest.read_text())
old = info["info"]["version"]
new = old + 1
text = manifest.read_text()
text, count = re.subn(rf'("version": ){old}\b', rf"\g<1>{new}", text, count=1)
assert count == 1, "could not find the version in source.json"
manifest.write_text(text)

readme = root / "README.md"
text = readme.read_text()
# Anchored on the source id, so another source whose label matches is left alone.
text, links = re.subn(rf"(sources/{re.escape(source_id)}-v){old}(\.aix)", rf"\g<1>{new}\g<2>", text)
text, labels = re.subn(
    rf"({re.escape(source_id)}-v{new}\.aix\">\.aix \(v){old}\)", rf"\g<1>{new})", text
)
if links != 2 or labels != 1:
    sys.exit(f"README row for {source_id} not updated cleanly ({links} links, {labels} labels)")
readme.write_text(text)
print(f"{source_id}: v{old} -> v{new}")
EOF
