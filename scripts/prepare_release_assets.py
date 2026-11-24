"""Stage the packages uploaded by one Release workflow run."""

import json
import os
import shutil
import sys
from pathlib import Path


def has_packages(directory):
    return any(
        path.is_file() and path.stat().st_size > 0
        and not path.name.endswith("-dist-manifest.json")
        for path in directory.glob("*")
    )


def main():
    source, destination, notes = map(Path, sys.argv[1:])
    plan = json.loads(
        (source / "artifacts-plan-dist-manifest" / "plan-dist-manifest.json").read_text()
    )
    matrix = (plan["ci"]["github"].get("artifacts_matrix") or {}).get("include") or []
    platforms = {
        ", ".join(entry["targets"]): source / (
            "artifacts-build-local-" + "_".join(entry["targets"])
        )
        for entry in matrix
    }
    platforms["Web"] = source / "artifacts-build-web"
    available = {name: path for name, path in platforms.items() if has_packages(path)}
    if not available:
        sys.exit("No platform installers or Web archive were uploaded")

    missing = sorted(platforms.keys() - available.keys())
    global_artifacts = source / "artifacts-build-global"
    complete = (
        not missing
        and has_packages(global_artifacts)
        and os.environ["BUILD_COMPLETE"] == "true"
    )

    destination.mkdir(parents=True, exist_ok=True)
    directories = list(available.values())
    if complete:
        directories.extend((global_artifacts, source / "artifacts-plan-dist-manifest"))
    for directory in directories:
        for path in sorted(directory.iterdir()):
            if not complete and path.name.endswith("-dist-manifest.json"):
                continue
            target = destination / path.name
            if not path.is_file() or target.exists():
                sys.exit(f"Unexpected or duplicate release asset: {path}")
            shutil.move(path, target)

    if not complete:
        lines = ["### 部分发版", "", "本次包含 " + "、".join(sorted(available)) + "。"]
        if missing:
            lines.append("缺少 " + "、".join(missing) + "。")
        lines.append("通用安装脚本、Homebrew 和 npm 随完整版本提供。")
        notes.write_text("\n".join(lines) + "\n", encoding="utf-8")

    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        print(f"complete={str(complete).lower()}", file=output)
        print(f"web={str('Web' in available).lower()}", file=output)


if __name__ == "__main__":
    main()
