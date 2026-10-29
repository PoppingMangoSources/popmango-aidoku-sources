"""Normalize fork-only tests out of an upstream source snapshot."""

import io
import re
import subprocess
import tarfile
import tomllib


def without_tests(files):
    files = dict(files)
    files.pop('src/test.rs', None)
    if 'src/lib.rs' in files:
        files['src/lib.rs'] = re.sub(rb'\n*#\[cfg\(test\)\]\nmod test;\n', b'\n', files['src/lib.rs'])
    manifest = files.get('Cargo.toml', b'').decode()
    if not manifest:
        return files
    info = tomllib.loads(manifest)
    rust = b'\n'.join(data for path, data in files.items() if path.endswith('.rs'))
    if 'serde_json' not in info.get('dev-dependencies', {}) or re.search(rb'\bserde_json\b', rust):
        return files
    sections = re.split(r'(?m)(?=^\[)', manifest)
    for i, section in enumerate(sections):
        if section.startswith('[dev-dependencies]\n'):
            sections[i] = re.sub(r'(?m)^serde_json\s*=.*\n', '', section)
    manifest = ''.join(sections)
    assert 'serde_json' not in tomllib.loads(manifest).get('dev-dependencies', {}), 'unsupported serde_json declaration'
    files['Cargo.toml'] = manifest.encode()
    if 'serde_json' not in info.get('dependencies', {}) and 'Cargo.lock' in files:
        packages = files['Cargo.lock'].decode().split('[[package]]')
        for i, package in enumerate(packages):
            if re.search(r'(?m)^name = "' + re.escape(info['package']['name']) + r'"$', package):
                packages[i] = re.sub(r'(?m)^ "serde_json",\n', '', package)
        files['Cargo.lock'] = '[[package]]'.join(packages).encode()
    return files


def snapshot_files(repo, ref, source):
    archive = subprocess.check_output(['git', '-C', str(repo), 'archive', ref, source])
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        files = {item.name[len(source) + 1:]: item.linkname.encode() if item.issym() else tar.extractfile(item).read()
                 for item in tar if item.isfile() or item.issym()}
    return files


def snapshot_modes(repo, ref, source):
    rows = subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', ref, '--', source], text=True)
    return {row.split('\t', 1)[1][len(source) + 1:]: row.split()[0] for row in rows.splitlines()}
