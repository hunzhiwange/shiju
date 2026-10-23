"""Focused packaging checks without accessing real signing credentials."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "bundle_macos.sh"
IDENTITY = "Developer ID Application: Fixture (TEAM123456)"
MOCK = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['CALL_LOG'], 'a') as log:
    log.write(json.dumps([name, *args]) + '\n')
if name == 'security' and args[0] == 'find-identity':
    print(os.environ['FIXTURE_IDENTITY'])
if name == 'xcrun' and args[:2] == ['notarytool', 'history']:
    sys.exit(int(os.environ.get('AUTH_FAILURE', '0')))
if name == 'xcrun' and args[:2] == ['notarytool', 'submit']:
    print(json.dumps({'status': 'Accepted'}))
if name == 'cargo':
    if args[:2] == ['bundle', '--help']:
        sys.exit(0)
    root = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
    if '--target' in args:
        root /= args[args.index('--target') + 1]
    root /= args[args.index('--profile') + 1] if '--profile' in args else 'release'
    if args[0] == 'bundle':
        with open(os.environ['CALL_LOG'], 'a') as log:
            log.write(json.dumps(['bundle-skip-build', os.environ.get('CARGO_BUNDLE_SKIP_BUILD')]) + '\n')
        app = root / 'bundle/osx/ShiJu.app/Contents'
        (app / 'MacOS').mkdir(parents=True)
        (app / 'Resources').mkdir()
        (app / 'Info.plist').write_text("""<?xml version="1.0"?>
<plist version="1.0"><dict><key>CFBundleExecutable</key><string>shi-ju</string>
<key>CFBundleIconFile</key><string>old.icns</string></dict></plist>""")
    else:
        (root / args[args.index('--bin') + 1]).write_text('fixture')
if name in ('ditto', 'hdiutil'):
    pathlib.Path(args[-1]).touch()
'''


class MacosSigningTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ("security", "xcrun", "cargo", "codesign", "ditto", "spctl", "hdiutil"):
            mock = self.bin / name
            mock.write_text(MOCK)
            mock.chmod(0o700)
        self.keychain = self.root / "signing.keychain-db"
        self.keychain.touch()
        self.password = self.root / "password"
        self.password.write_text("fixture-password")
        self.password.chmod(0o600)
        self.icon = self.root / "icon.icns"
        self.icon.touch()
        self.log = self.root / "calls.jsonl"
        self.env = {
            "PATH": f"{self.bin}:{os.environ['PATH']}",
            "HOME": os.environ["HOME"],
            "CALL_LOG": str(self.log),
            "FIXTURE_IDENTITY": IDENTITY,
            "APPLE_SIGNING_IDENTITY": IDENTITY,
            "SHIJU_NOTARY_PROFILE": "shiju-notary",
            "SHIJU_NOTARY_KEYCHAIN": str(self.keychain),
            "SHIJU_NOTARY_KEYCHAIN_PASSWORD_FILE": str(self.password),
            "MACOS_SIGN_MODE": "developer-id",
            "MACOS_NOTARIZE": "1",
            "PACKAGE_APP": "1",
            "CREATE_DMG": "1",
            "CARGO_TARGET_DIR": str(self.root / "target"),
            "OUT_DIR": str(self.root / "dist"),
            "MACOS_ICON_SOURCE": str(self.icon),
        }

    def run_bundle(self):
        result = subprocess.run(
            ["bash", str(SCRIPT)], cwd=self.root, env=self.env,
            capture_output=True, text=True, check=False,
        )
        calls = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        return result, calls

    def test_shared_keychain_signs_app_and_dmg_before_notarization(self):
        result, calls = self.run_bundle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        signatures = [c for c in calls if c[0] == "codesign" and "--sign" in c]
        self.assertEqual(len(signatures), 4)  # Helper, CLI, app, DMG.
        for call in signatures:
            self.assertEqual(call[call.index("--keychain") + 1], str(self.keychain))
            self.assertEqual(call[call.index("--sign") + 1], IDENTITY)
        dmg_sign = signatures[-1]
        dmg_submit = next(c for c in calls if c[:3] == ["xcrun", "notarytool", "submit"] and c[3].endswith(".dmg"))
        self.assertLess(calls.index(dmg_sign), calls.index(dmg_submit))
        history = next(c for c in calls if c[:3] == ["xcrun", "notarytool", "history"])
        self.assertLess(calls.index(history), next(i for i, c in enumerate(calls) if c[0] == "cargo"))

    def test_missing_keychain_fails_before_build(self):
        self.keychain.unlink()
        result, calls = self.run_bundle()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("keychain not found", result.stderr)
        self.assertFalse(any(c[0] == "cargo" for c in calls))

    def prepare_prebuilt_binaries(self):
        self.env.update(SKIP_BUILD="1", PROFILE="dist", TARGET="aarch64-apple-darwin",
                        INCLUDE_ACP_IN_APP="1", INCLUDE_CLI_COMPAT_IN_APP="1")
        root = Path(self.env["CARGO_TARGET_DIR"]) / self.env["TARGET"] / "dist"
        root.mkdir(parents=True)
        for name in ("shi-ju", "vw-webview", "shiju", "vibe-agent", "acp"):
            binary = root / name
            binary.write_text(f"prebuilt {name}")
            binary.chmod(0o755)
        return root

    def test_prebuilt_binaries_are_packaged_without_compiling(self):
        root = self.prepare_prebuilt_binaries()
        result, calls = self.run_bundle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(any(c[:2] == ["cargo", "build"] for c in calls))
        self.assertIn(["bundle-skip-build", "1"], calls)
        bundle_call = next(c for c in calls if c[:2] == ["cargo", "bundle"] and "--bin" in c)
        self.assertEqual(bundle_call[bundle_call.index("--format") + 1], "osx")
        bundled = root / "bundle/osx/ShiJu.app/Contents/MacOS"
        for name in ("vw-webview", "shiju", "vibe-agent", "acp"):
            self.assertEqual((bundled / name).read_bytes(), (root / name).read_bytes())
        self.assertTrue(list(Path(self.env["OUT_DIR"]).glob("*.dmg")))
        self.assertTrue(list(Path(self.env["OUT_DIR"]).glob("*.zip")))

    def test_missing_prebuilt_binary_fails_without_compiling(self):
        root = self.prepare_prebuilt_binaries()
        for name in ("shi-ju", "vw-webview", "shiju", "vibe-agent", "acp"):
            with self.subTest(binary=name):
                binary = root / name
                binary.chmod(0o644)
                result, calls = self.run_bundle()
                binary.chmod(0o755)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"expected prebuilt executable not found: {binary}", result.stderr)
                self.assertFalse(any(c[:2] == ["cargo", "build"] for c in calls))
                self.assertFalse(any(c[0] == "bundle-skip-build" for c in calls))

    def test_invalid_notary_credentials_fail_before_build(self):
        self.env["AUTH_FAILURE"] = "1"
        result, calls = self.run_bundle()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("notarization credentials are unavailable", result.stderr)
        self.assertFalse(any(c[0] == "cargo" for c in calls))

    def test_explicit_configuration_overrides_shared_variables(self):
        self.env.update(MACOS_SIGN_IDENTITY=IDENTITY, NOTARY_KEYCHAIN_PROFILE="explicit",
                        NOTARY_KEYCHAIN_PATH=str(self.keychain),
                        NOTARY_KEYCHAIN_PASSWORD_FILE=str(self.password),
                        APPLE_SIGNING_IDENTITY="ignored", SHIJU_NOTARY_KEYCHAIN="missing",
                        SHIJU_NOTARY_KEYCHAIN_PASSWORD_FILE="missing")
        result, calls = self.run_bundle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        for call in calls:
            if call[:2] == ["xcrun", "notarytool"]:
                self.assertEqual(call[call.index("--keychain-profile") + 1], "explicit")

    def test_default_keychain_does_not_add_empty_codesign_argument(self):
        del self.env["SHIJU_NOTARY_KEYCHAIN"]
        del self.env["SHIJU_NOTARY_KEYCHAIN_PASSWORD_FILE"]
        result, calls = self.run_bundle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        for call in calls:
            if call[0] == "codesign" and "--sign" in call:
                self.assertNotIn("--keychain", call)
                self.assertNotIn("", call)


if __name__ == "__main__":
    unittest.main()
