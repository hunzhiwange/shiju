"""Fail closed unless every in-scope frozen plugin capability has evidence."""
import argparse
import json
from pathlib import Path
import re
import sys

PLATFORMS = ["macos", "linux", "windows"]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def object_value(value, label):
    require(isinstance(value, dict), f"{label}: expected object")
    return value


def text(value, label):
    require(isinstance(value, str) and value.strip(), f"{label}: expected nonempty text")
    return value


def array(value, label):
    require(isinstance(value, list), f"{label}: expected array")
    return value


def names(value, label):
    values = array(value, label)
    require(values, f"{label}: cannot be empty")
    for entry in values:
        text(entry, label)
    require(len(set(values)) == len(values), f"{label}: duplicate value")
    return values


def tasks(value, label):
    values = array(value, label)
    require(values and all(type(v) is int and v > 0 for v in values),
            f"{label}: expected positive task numbers")
    require(len(set(values)) == len(values), f"{label}: duplicate task")
    return values


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load(path):
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    except (json.JSONDecodeError, UnicodeError) as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    value = object_value(value, str(path))
    require(type(value.get("schema_version")) is int and value["schema_version"] == 1,
            f"{path}: unsupported schema_version")
    return value


def indexed_items(document, label):
    result = {}
    for entry in array(document.get("items"), f"{label}.items"):
        entry = object_value(entry, f"{label}.item")
        identifier = text(entry.get("id"), f"{label}.id")
        require(identifier not in result, f"{label}: duplicate id {identifier}")
        result[identifier] = entry
    return result


def check_record(record, label, succeeds):
    record = object_value(record, label)
    text(record.get("command"), f"{label}.command")
    text(record.get("record"), f"{label}.record")
    code = record.get("exit_code")
    count = record.get("effective_tests")
    require(type(code) is int and code >= 0, f"{label}.exit_code: expected nonnegative integer")
    require((code == 0) if succeeds else (code != 0), f"{label}: incorrect exit_code")
    require(type(count) is int and count > 0, f"{label}.effective_tests: zero or invalid tests")
    return record


def validate(baseline, evidence):
    sources = object_value(baseline.get("sources"), "baseline.sources")
    require(sources, "baseline.sources: cannot be empty")
    for key, source in sources.items():
        source = object_value(source, f"source {key}")
        sha = source.get("sha256")
        require(isinstance(sha, str) and re.fullmatch(r"[0-9a-f]{64}", sha),
                f"source {key}: invalid sha256")
    baseline_items = indexed_items(baseline, "baseline")
    require(baseline_items, "baseline.items: cannot be empty")
    evidence_items = indexed_items(evidence, "evidence")
    unknown = evidence_items.keys() - baseline_items.keys()
    require(not unknown, f"evidence: unknown ids {sorted(unknown)}")
    included = 0
    for identifier, item in baseline_items.items():
        label = f"{identifier}"
        required_tasks = tasks(item.get("tasks"), f"{label}.tasks")
        references = array(item.get("sources"), f"{label}.sources")
        require(references, f"{label}: missing reference symbols")
        for reference in references:
            reference = object_value(reference, f"{label}.source")
            source = reference.get("source")
            require(isinstance(source, str) and source in sources, f"{label}: unknown source")
            text(reference.get("symbol"), f"{label}.source.symbol")
            require(reference.get("sha256") == sources[source]["sha256"],
                    f"{label}: source hash mismatch")
        in_scope = item.get("in_scope", item.get("basis") != "excluded_execution")
        require(type(in_scope) is bool, f"{label}.in_scope: expected boolean")
        if not in_scope:
            continue
        included += 1
        require(identifier in evidence_items, f"{label}: missing evidence item")
        record = evidence_items[identifier]
        require(record.get("conclusion") == "verified", f"{label}: conclusion is pending or unverified")
        require(set(required_tasks) <= set(tasks(record.get("tasks"), f"{label}.evidence.tasks")),
                f"{label}: missing mapped task evidence")
        names(record.get("tests"), f"{label}.tests")
        red = check_record(record.get("red"), f"{label}.red", succeeds=False)
        green = check_record(record.get("green"), f"{label}.green", succeeds=True)
        require(red["command"] == green["command"], f"{label}: red/green commands differ")
        required_platforms = names(item.get("required_platforms", baseline.get("required_platforms", PLATFORMS)),
                                   f"{label}.required_platforms")
        platforms = object_value(record.get("platforms"), f"{label}.platforms")
        verified = names(platforms.get("verified"), f"{label}.platforms.verified")
        require(set(required_platforms) <= set(PLATFORMS) and set(verified) <= set(PLATFORMS),
                f"{label}: unknown platform")
        require(set(required_platforms) <= set(verified), f"{label}: unverified platform")
        prerequisites = array(record.get("external_prerequisites"), f"{label}.external_prerequisites")
        for prerequisite in prerequisites:
            prerequisite = object_value(prerequisite, f"{label}.prerequisite")
            text(prerequisite.get("name"), f"{label}.prerequisite.name")
            text(prerequisite.get("record"), f"{label}.prerequisite.record")
            require(prerequisite.get("status") == "verified", f"{label}: unverified external prerequisite")
            require(prerequisite.get("mode") == "real", f"{label}: mock external prerequisite")
        if set(required_tasks) & {62, 63}:
            require(prerequisites, f"{label}: missing real public-service verification")
    require(included > 0, "baseline: no in-scope capabilities")
    return included


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--evidence", required=True)
    args = parser.parse_args()
    try:
        count = validate(load(args.baseline), load(args.evidence))
    except (OSError, ValueError) as error:
        print(f"Evidence rejected: {error}", file=sys.stderr)
        return 1
    print(f"Evidence complete for {count} in-scope baseline items.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
