"""Exercise the real bundle script with only build/signing boundaries replaced."""

from pathlib import Path
import plistlib
import unittest

import test_macos_signing


@unittest.skipUnless(Path("/usr/libexec/PlistBuddy").exists(), "Requires macOS plist tools")
class VoicePackagingTests(unittest.TestCase):
    def bundle(self, mode, existing_description=False, profile="release"):
        fixture = test_macos_signing.MacosSigningTests(methodName="runTest")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        fixture.env.update(
            SKIP_BUILD="1", MACOS_SIGN_MODE=mode, MACOS_NOTARIZE="0",
            PACKAGE_APP="0", CREATE_DMG="0", INCLUDE_CLI_IN_APP="0", PROFILE=profile,
        )
        root = Path(fixture.env["CARGO_TARGET_DIR"]) / ("debug" if profile == "dev" else profile)
        root.mkdir(parents=True)
        for name in ("shi-ju", "vw-webview"):
            binary = root / name
            binary.write_text("prebuilt fixture")
            binary.chmod(0o755)
        if existing_description:
            cargo = fixture.bin / "cargo"
            cargo.write_text(cargo.read_text().replace(
                "<key>CFBundleIconFile</key>",
                "<key>NSMicrophoneUsageDescription</key><string>Old website-only usage</string>"
                "<key>CFBundleIconFile</key>",
            ))
        result, calls = fixture.run_bundle()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        # These fixtures must never reach publication or notarization boundaries.
        self.assertFalse(any(c[0] in ("xcrun", "ditto", "hdiutil", "spctl") for c in calls))
        self.assertFalse(any(c[:2] == ["cargo", "build"] for c in calls))
        self.assertIn(["bundle-skip-build", "1"], calls)
        bundle = root / "bundle/osx/ShiJu.app"
        return plistlib.loads((bundle / "Contents/Info.plist").read_bytes()), calls

    def test_voice_bundle_describes_native_microphone_usage(self):
        for mode in ("developer-id", "adhoc"):
            for existing in (False, True):
                with self.subTest(mode=mode, existing=existing):
                    info, calls = self.bundle(mode, existing)
                    usage = info["NSMicrophoneUsageDescription"].lower()
                    for purpose in ("dictation", "voice chat", "websites you allow"):
                        self.assertIn(purpose, usage)
                    for call in calls:
                        if call[0] == "media-signing":
                            self.assertEqual(call[2][1], info["NSMicrophoneUsageDescription"])
                    self.assertFalse(any(k in info for k in (
                        "NSAccessibilityUsageDescription", "NSInputMonitoringUsageDescription",
                        "NSScreenCaptureUsageDescription",
                    )))

    def test_voice_bundle_uses_audio_input_entitlement(self):
        _, calls = self.bundle("developer-id")
        helper = next(c for c in calls if c[:2] == ["media-signing", "vw-webview"])
        self.assertEqual(helper[3], {
            "com.apple.security.device.camera": True,
            "com.apple.security.device.audio-input": True,
        })

    def test_voice_dev_profile_packages_debug_artifacts(self):
        self.bundle("adhoc", profile="dev")

    def test_native_voice_process_has_only_audio_input_entitlement(self):
        for mode in ("developer-id", "adhoc"):
            with self.subTest(mode=mode):
                _, calls = self.bundle(mode)
                app = next(c for c in calls if c[:2] == ["media-signing", "ShiJu.app"])
                self.assertEqual(app[3], {"com.apple.security.device.audio-input": True})
                helper = next(c for c in calls if c[:2] == ["media-signing", "vw-webview"])
                self.assertEqual(helper[3], {
                    "com.apple.security.device.camera": True,
                    "com.apple.security.device.audio-input": True,
                })
