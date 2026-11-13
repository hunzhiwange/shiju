"""Check patched Relax against two vendored versions and a missing-version case."""
import os
from pathlib import Path
import subprocess
import tempfile

import relax_dev


def main():
    binary, tool, _ = relax_dev.prepare()
    with tempfile.TemporaryDirectory(prefix='shiju-relax-vendor-') as folder:
        root = Path(folder)
        files = {
            'Cargo.toml': '[workspace]\nmembers=["app"]\nresolver="2"\n',
            'app/Cargo.toml': (
                '[package]\nname="app"\nversion="0.1.0"\nedition="2021"\n'
                '[dependencies]\nold={package="probe",version="0.1"}\n'
                'new={package="probe",version="0.2"}\n'
            ),
            'app/src/lib.rs': (
                "pub fn old_value()->usize {old::value()}\n"
                "pub fn new_value()->&'static str {new::value()}\n"
            ),
            '.cargo/config.toml': (
                '[source.crates-io]\nreplace-with="v"\n'
                '[source.v]\ndirectory="vendor"\n'
            ),
            'Cargo.lock': 'version = 4\n',
        }
        for version, source in [
            ('0.1.0', 'pub fn value()->usize {1}'),
            ('0.2.0', "pub fn value()->&'static str {\"new\"}"),
        ]:
            files['Cargo.lock'] += (
                f'[[package]]\nname="probe"\nversion="{version}"\n'
                'source="registry+https://github.com/rust-lang/crates.io-index"\n'
            )
            prefix = f'vendor/probe-{version}'
            files[f'{prefix}/Cargo.toml'] = (
                f'[package]\nname="probe"\nversion="{version}"\nedition="2021"\n'
            )
            files[f'{prefix}/src/lib.rs'] = source
            files[f'{prefix}/.cargo-checksum.json'] = '{"files":{},"package":null}'
        for relative, contents in files.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        env = os.environ.copy()
        env.update(
            MAKEPAD_RUN_STD=str(tool / 'libs/relax/compiler/std'),
            CARGO_TARGET_DIR=str(root / 'target'),
            CARGO_BUILD_BUILD_DIR=str(root / 'build'),
        )
        command = [binary, 'check', 'app', '--lib']
        subprocess.run(command, cwd=root, env=env, check=True)
        print('PASS: two required versions selected with distinct return types', flush=True)
        manifest = root / 'app/Cargo.toml'
        manifest.write_text(manifest.read_text().replace('version="0.1"', 'version="9"'))
        result = subprocess.run(command, cwd=root, env=env)
        if result.returncode == 0:
            raise RuntimeError('Relax incorrectly accepted an unavailable dependency version')
        print('PASS: unavailable required version rejected', flush=True)


if __name__ == '__main__':
    main()
