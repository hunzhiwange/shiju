#!/usr/bin/env python3
"""Package wasm-bindgen's complete ShiJu application as a standalone static ZIP."""

import argparse
import hashlib
from pathlib import Path
import re
import shutil
import tempfile
import zipfile

SCRIPTS = Path(__file__).resolve().parent


def write_index(dist):
    template = SCRIPTS.parent / 'crates/vw-desktop/index.html'
    html = re.sub(r'\s*<(?:base|link)\b[^>]*\bdata-trunk[^>]*>', '', template.read_text(encoding='utf-8'))
    # Initializing wasm-bindgen already calls the binary entry point once.
    bootstrap = """<script type="module">
import init from './shi_ju.js';
init().catch(error => {
    window.shijuInitError = String(error);
    console.error(error);
});
</script>"""
    (dist / 'index.html').write_text(html.replace('</body>', bootstrap + '\n</body>'), encoding='utf-8')


def package(bindgen_dir, out_dir, version):
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?', version):
        raise ValueError('Invalid package version')
    bindgen_dir = Path(bindgen_dir).resolve()
    out_dir = Path(out_dir).resolve()
    for name in ('shi_ju.js', 'shi_ju_bg.wasm'):
        if not (bindgen_dir / name).is_file():
            raise ValueError(f'Missing wasm-bindgen output: {name}')
    files = []
    for path in sorted(bindgen_dir.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'Refusing symlink in wasm-bindgen output: {path.name}')
        if not path.is_file():
            continue
        relative = path.relative_to(bindgen_dir)
        if relative.as_posix() == 'index.html':
            continue  # Regenerate the developer entry point for the archive.
        if relative.parts[0] != 'snippets' and relative.as_posix() not in {
            'shi_ju.js', 'shi_ju_bg.wasm', 'shi_ju.d.ts', 'shi_ju_bg.wasm.d.ts',
        }:
            raise ValueError(f'Unexpected wasm-bindgen output: {relative}')
        files.append((path, relative))
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = f'shiju-web-{version}'
    archive = out_dir / (stem + '.zip')
    with tempfile.TemporaryDirectory(prefix='shiju-web-') as temp:
        stage = Path(temp) / stem
        stage.mkdir()
        for source, relative in files:
            destination = stage / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
        write_index(stage)
        shutil.copyfile(SCRIPTS / 'web/serve.mjs', stage / 'serve.mjs')
        manifest = (SCRIPTS / 'web/package.json').read_text(encoding='utf-8')
        (stage / 'package.json').write_text(manifest.replace('@VERSION@', version), encoding='utf-8')
        readme = (SCRIPTS / 'web/README.md').read_text(encoding='utf-8')
        (stage / 'README.md').write_text(readme.replace('@VERSION@', version), encoding='utf-8')
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
            for path in sorted(stage.rglob('*')):
                if path.is_file():
                    bundle.write(path, path.relative_to(stage.parent).as_posix())
    digest = hashlib.sha256()
    with archive.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    archive.with_suffix('.zip.sha256').write_text(f'{digest.hexdigest()}  {archive.name}\n', encoding='ascii')
    return archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bindgen-dir', type=Path, required=True)
    parser.add_argument('--out-dir', type=Path, default=Path('target/distrib'))
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    try:
        print(package(args.bindgen_dir, args.out_dir, args.version))
    except ValueError as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()
