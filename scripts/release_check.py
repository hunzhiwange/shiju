#!/usr/bin/env python3
"""Check release binaries without turning Cargo warnings into errors."""
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

NATIVE_PACKAGES = ("vw-desktop", "vw-webview", "vw-cli", "vw-acp")
WASM_TARGET = "wasm32-unknown-unknown"
LINUX_ARCHES = {"aarch64": "arm64", "x86_64": "amd64"}
LINUX_PACKAGES = (
    "g++", "pkg-config", "libasound2-dev", "libfontconfig1-dev", "libgtk-3-dev",
    "libwebkit2gtk-4.1-dev", "libxkbcommon-dev",
)


def diagnostic_lines(lines):
    for line in lines:
        stripped = line.lstrip()
        if stripped.startswith(("warning:", "cargo:warning=")):
            # cc-rs also routes native compiler errors through cargo:warning.
            if re.search(r"\b(?:fatal )?error(?:\[[A-Z]+\d+\])?:", stripped):
                yield line.replace("cargo:warning=", "", 1)
        else:
            yield line


def quiet_command(command, **kwargs):
    result = subprocess.run(command, stderr=subprocess.PIPE, **kwargs)
    if result.returncode:
        detail = "".join(diagnostic_lines(result.stderr.decode(errors="replace").splitlines(keepends=True))).strip()
        raise RuntimeError(detail or f"{command[0]} exited with {result.returncode}")
    return result.stdout


def linux_environment(target, environment):
    arch = target.split("-", 1)[0]
    docker_arch = LINUX_ARCHES[arch]
    cache = Path("target/release-check").resolve()
    cache.mkdir(parents=True, exist_ok=True)
    sysroot = cache / f"sysroot-{docker_arch}"
    multiarch = f"{arch}-linux-gnu"
    libdirs = (
        f"{sysroot}/usr/lib/{multiarch}/pkgconfig:{sysroot}/usr/lib/pkgconfig:"
        f"{sysroot}/usr/share/pkgconfig"
    )
    llvm = {tool: shutil.which(tool) for tool in ("clang", "clang++", "llvm-ar", "llvm-ranlib")}
    missing = [tool for tool, path in llvm.items() if path is None]
    if missing:
        raise RuntimeError(f"LLVM tools required on PATH: {', '.join(missing)}")
    if not (sysroot / f"usr/lib/{multiarch}/pkgconfig/webkit2gtk-4.1.pc").is_file():
        # Extract the same Ubuntu dependencies used by the release runners. No
        # source checkout is mounted into Docker, and Cargo keeps its host cache.
        with tempfile.TemporaryDirectory(dir=cache) as scratch, tempfile.TemporaryFile() as archive:
            quiet_command(
                ["docker", "run", "--rm", "--platform", f"linux/{docker_arch}", "ubuntu:22.04",
                 "sh", "-ec", 'apt-get update -qq >&2 && '
                 'DEBIAN_FRONTEND=noninteractive apt-get install -qq -y --no-install-recommends "$@" >&2 && '
                 'tar -C / -cf - usr/include usr/lib usr/share/pkgconfig', "sh", *LINUX_PACKAGES],
                stdout=archive,
            )
            archive.seek(0)
            staged = Path(scratch) / "sysroot"
            staged.mkdir()
            quiet_command(["tar", "-xf", "-", "-C", staged], stdin=archive, stdout=subprocess.DEVNULL)
            (staged / "lib").symlink_to("usr/lib")
            try:
                staged.rename(sysroot)
            except OSError:
                # Another release check may have populated the shared cache.
                if not (sysroot / f"usr/lib/{multiarch}/pkgconfig/webkit2gtk-4.1.pc").is_file():
                    raise
    target_vars = {
        "PKG_CONFIG_ALLOW_CROSS": "1",
        "PKG_CONFIG_SYSROOT_DIR": str(sysroot),
        "PKG_CONFIG_LIBDIR": libdirs,
        "PKG_CONFIG_PATH": "",
        "CC": llvm["clang"],
        "CXX": llvm["clang++"],
        "AR": llvm["llvm-ar"],
        "RANLIB": llvm["llvm-ranlib"],
        "CFLAGS": f"--sysroot={sysroot}",
        "CXXFLAGS": f"--sysroot={sysroot} --gcc-toolchain={sysroot}/usr",
        "BINDGEN_EXTRA_CLANG_ARGS": f"--sysroot={sysroot}",
    }
    libclang = Path(llvm["clang"]).resolve().parent.parent / "lib"
    if any(libclang.glob("libclang.*")):
        environment.setdefault("LIBCLANG_PATH", str(libclang))
    suffix = target.replace("-", "_")
    return environment | {f"{key}_{suffix}": value for key, value in target_vars.items()}


def cargo_check(command, environment):
    with tempfile.TemporaryFile(mode="w+t", encoding="utf-8", errors="replace") as stderr:
        with subprocess.Popen(
            command, env=environment, stdout=subprocess.PIPE, stderr=stderr,
            text=True, encoding="utf-8", errors="replace",
        ) as cargo:
            for line in cargo.stdout:
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    if line.startswith(("error:", "error[")):
                        print(line, end="", file=sys.stderr)
                    continue
                if event.get("reason") == "compiler-message":
                    message = event["message"]
                    if message["level"] == "error":
                        print(message.get("rendered") or message["message"], end="\n", file=sys.stderr)
            status = cargo.wait()
        if status:
            stderr.seek(0)
            sys.stderr.writelines(diagnostic_lines(stderr))
    return status


def check_target(target, installed_targets):
    environment = os.environ | {"CARGO_TERM_COLOR": "never", "VIBEWINDOW_RELEASE_BUILD": "1"}
    prefix = ["cargo"]
    if target.endswith("-apple-darwin"):
        if platform.system() != "Darwin":
            raise RuntimeError("a macOS host with the Apple SDK is required for this target")
        quiet_command(["xcrun", "--show-sdk-path"], stdout=subprocess.DEVNULL)
    elif target.endswith("-pc-windows-msvc") and platform.system() != "Windows":
        prefix += ["xwin"]
    elif target.endswith("-unknown-linux-gnu"):
        environment = linux_environment(target, environment)

    if target not in installed_targets:
        quiet_command(["rustup", "target", "add", target], stdout=subprocess.DEVNULL)

    options = ["--locked", "--quiet", "--profile", "dist", "--target", target, "--message-format=json"]
    if target == WASM_TARGET:
        return not cargo_check([*prefix, "build", *options, "-p", "vw-desktop", "--bin", "shi-ju"], environment)

    if prefix[-1] == "xwin" and target.startswith("aarch64-"):
        options += ["--cross-compiler", "clang-cl"]
        # ring invokes clang for ARM64 assembly even with the clang-cl backend.
        # Rewrite xwin's /imsvc to an include flag both drivers understand.
        environment["CCC_OVERRIDE_OPTIONS"] = (
            f"{environment.get('CCC_OVERRIDE_OPTIONS', '#')} s/^.imsvc$/-I/"
        )
    packages = NATIVE_PACKAGES
    if target.endswith("-apple-darwin"):
        packages += ("computer-control",)
    failed = False
    for package in packages:
        binaries = ["--bins", "--all-features"]
        if package == "computer-control":
            binaries = ["--bin", "shiju-computer-use"]
        command = [*prefix, "check", *options, "-p", package, *binaries, "--keep-going"]
        status = cargo_check(command, environment)
        if status:
            print(f"error: {target}: {package} Cargo check exited with {status}", file=sys.stderr)
            failed = True
    return not failed


def main(targets):
    if not targets:
        print("error: no release targets specified", file=sys.stderr)
        return 1
    try:
        installed = quiet_command(["rustup", "target", "list", "--installed"], stdout=subprocess.PIPE)
        installed_targets = set(installed.decode().splitlines())
    except (OSError, RuntimeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    failed = []
    for target in targets:
        print(f"Checking {target}", flush=True)
        try:
            passed = check_target(target, installed_targets)
        except (OSError, RuntimeError) as exc:
            print(f"error: {target}: {exc}", file=sys.stderr)
            passed = False
        if not passed:
            failed.append(target)
    if failed:
        print(f"error: release checks failed for {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"Release checks passed for {len(targets)} targets.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
