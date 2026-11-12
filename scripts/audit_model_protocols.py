#!/usr/bin/env python3
"""Inventory source declarations; audit research coverage without network or credentials."""
import argparse
from collections import Counter
from datetime import date
import hashlib
import json
from pathlib import Path
import sys

FIELDS = ("api", "npm", "adapter", "shape")
CAPABILITIES = ("text", "tools", "thinking", "media", "streaming", "structured_output")
CONCLUSIONS = {"官方确认", "实测通过", "明确不支持", "资料不足", "不适用于文本ACP", "缺少凭据", "实现缺口"}
PROTOCOLS = {"chat", "messages", "responses", "gemini"}
ROOT = Path(__file__).resolve().parents[1]
INITIAL_BASELINE = ROOT / ".plans/2026-11-11-全量模型协议适配与核查/目录基线.json"


def load_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(Path(path).read_text(encoding="utf-8"), object_pairs_hook=unique)


def pointer(*parts):
    return "/" + "/".join(str(p).replace("~", "~0").replace("/", "~1") for p in parts)


def source_name(path):
    path = Path(path)
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def declarations(value):
    return {k: value[k] for k in FIELDS if k in value}


def build_inventory(sources):
    """sources are ordered [api, shiju]; model overrides are declarations, not routes."""
    if len(sources) != 2 or Path(sources[0]).resolve() == Path(sources[1]).resolve():
        raise ValueError("two distinct sources required: api then shiju")
    catalogs = [load_json(p) for p in sources]
    result = {"schema_version": 1, "sources": {}, "providers": [], "records": []}
    for index, (path, catalog) in enumerate(zip(sources, catalogs)):
        if not isinstance(catalog, dict):
            raise ValueError("catalog must be a provider map")
        source = source_name(path)
        result["sources"][source] = {
            "sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest(),
            "providers": len(catalog), "models": 0}
        for provider_id, provider in sorted(catalog.items()):
            provider_decl = declarations(provider)
            state = "active"
            if index == 0 and provider_id == "shiju" and "shiju" in catalogs[1]:
                state = "shadowed_by_shiju"
            elif index == 1 and provider_id != "shiju":
                state = "overlay_provider_not_loaded"
            models = provider.get("models", {})
            result["sources"][source]["models"] += len(models)
            result["providers"].append({
                "source": source, "provider_id": provider_id,
                "declared_id": provider.get("id"), "model_count": len(models),
                "declarations": provider_decl, "doc": provider.get("doc"),
                "env": provider.get("env", []), "runtime_catalog_state": state})
            for model_key, model in sorted(models.items()):
                override = model.get("provider") or {}
                effective = {**provider_decl, **declarations(override)}
                gaps = []
                if not effective.get("api"):
                    gaps.append("default_endpoint_unverified")
                if any(k in provider_decl for k in ("npm", "shape")):
                    gaps.append("provider_protocol_fields_not_loaded")
                if any(k in override for k in ("api", "npm", "shape")):
                    gaps.append("model_provider_fields_not_loaded")
                if not any(effective.get(k) for k in ("npm", "adapter")):
                    gaps.append("protocol_declaration_missing")
                if effective.get("npm") and effective.get("adapter"):
                    gaps.append("npm_adapter_precedence_requires_review")
                modalities = model.get("modalities") or {}
                if not modalities:
                    applicability = "modalities_unknown"
                elif "text" not in modalities.get("output", []):
                    applicability = "no_text_output"
                elif str(effective.get("api", "")).startswith(("wss:", "ws:")):
                    applicability = "specialized_transport_review"
                else:
                    applicability = "text_candidate_requires_protocol_review"
                result["records"].append({
                    "source": source, "provider_id": provider_id, "model_key": model_key,
                    "json_pointer": pointer(provider_id, "models", model_key),
                    "model_api_id": model.get("id", ""),
                    "canonical_model_id": model.get("canonical_model_id"),
                    "provider_declarations": provider_decl,
                    "model_provider": override, "effective_declarations": effective,
                    "capabilities": {k: model[k] for k in (
                        "modalities", "reasoning", "reasoning_options", "tool_call",
                        "structured_output", "interleaved", "status", "limit") if k in model},
                    "runtime_catalog_state": state,
                    "text_acp_applicability": applicability,
                    "metadata_gaps": gaps, "verification_status": "未核查"})
    result["provider_count"] = len({p["provider_id"] for p in result["providers"]})
    result["provider_record_count"] = len(result["providers"])
    result["model_record_count"] = len(result["records"])
    return result


def build_groups(inventory, seed):
    """Keep baseline assignments; assign additions to the least populated group."""
    original = seed.get("baseline_groups", seed.get("research_groups", seed.get("groups", [])))
    if not original:
        raise ValueError("research group seed is empty")
    seen = set()
    group_ids = set()
    for group in original:
        if group["group"] in group_ids:
            raise ValueError("duplicate group ID")
        group_ids.add(group["group"])
        for provider in group["providers"]:
            if provider in seen:
                raise ValueError(f"duplicate provider assignment: {provider}")
            seen.add(provider)
    current = {p["provider_id"] for p in inventory["providers"]}
    groups = [{"group": g["group"], "task": g["task"],
               "providers": sorted(set(g["providers"]) & current)} for g in original]
    for provider in sorted(current - seen):
        min(groups, key=lambda g: (len(g["providers"]), g["group"]))["providers"].append(provider)
    for group in groups:
        group["providers"].sort()
        group["provider_count"] = len(group["providers"])
        group["model_count"] = sum(r["provider_id"] in group["providers"]
                                   for r in inventory["records"])
    baseline_sources = seed.get("baseline_sources", seed.get("sources", {}))
    return {"schema_version": 1, "baseline_groups": original,
            "baseline_sources": baseline_sources,
            "source_hashes": {s: v["sha256"] for s, v in inventory["sources"].items()},
            "source_changes": {s: {"baseline": baseline_sources.get(s), "current": v}
                               for s, v in inventory["sources"].items()
                               if baseline_sources.get(s) != v},
            "added_providers": sorted(current - seen),
            "removed_providers": sorted(seen - current), "groups": groups}


def identity(row):
    return tuple(row.get(k) for k in ("source", "provider_id", "model_key"))


def nonempty(value):
    return isinstance(value, str) and bool(value.strip())


def check_research(inventory, report, group):
    """Return contract errors. Passing proves coverage, never truth of external claims."""
    errors = []
    if not isinstance(report, dict):
        return ["report must be an object"]
    if report.get("schema_version") != 1 or report.get("group") != group:
        errors.append("schema_version/group mismatch")
    assigned = next((g["providers"] for g in inventory.get("research_groups", [])
                     if g["group"] == group), None)
    if assigned is None:
        return errors + ["group missing from inventory"]
    expected_hashes = {s: v["sha256"] for s, v in inventory["sources"].items()}
    if report.get("source_hashes") != expected_hashes:
        errors.append("source_hashes conflict")
    providers = report.get("providers", [])
    if not isinstance(providers, list) or sorted(providers) != sorted(assigned):
        errors.append("provider coverage mismatch")
    evidence = {}
    items = report.get("evidence", [])
    if not isinstance(items, list):
        return errors + ["evidence must be an array"]
    for item in items:
        if not isinstance(item, dict) or not nonempty(item.get("id")):
            errors.append("invalid evidence object/id")
            continue
        eid = item["id"]
        if eid in evidence:
            errors.append(f"duplicate evidence: {eid}")
        evidence[eid] = item
        if item.get("type") not in {"official_docs", "official_sdk", "official_query", "real_call", "offline"}:
            errors.append(f"invalid evidence type: {eid}")
        if not nonempty(item.get("url")) or not item["url"].startswith(("https://", "http://")):
            errors.append(f"missing query/source URL: {eid}")
        if not nonempty(item.get("excerpt")):
            errors.append(f"missing evidence excerpt: {eid}")
        try:
            checked = date.fromisoformat(item.get("checked_at", ""))
            if checked > date.today():
                errors.append(f"future evidence date: {eid}")
        except (ValueError, TypeError):
            errors.append(f"invalid evidence date: {eid}")
    expected = {identity(r): r for r in inventory["records"] if r["provider_id"] in assigned}
    seen = set()
    rows = report.get("records", [])
    if not isinstance(rows, list):
        return errors + ["records must be an array"]
    for row in rows:
        if not isinstance(row, dict):
            errors.append("invalid research record")
            continue
        key = identity(row)
        if not all(isinstance(part, str) for part in key):
            errors.append("invalid source identity")
            continue
        if key in seen or key not in expected:
            errors.append(f"duplicate or unexpected source row: {key}")
        seen.add(key)
        if key in expected and any(k not in row or row[k] != expected[key][k]
                                   for k in ("model_api_id", "canonical_model_id")):
            errors.append(f"model identity conflict: {key}")
        conclusion = row.get("conclusion")
        if conclusion not in CONCLUSIONS:
            errors.append(f"invalid conclusion: {key}")
        for field in ("reason", "auth", "deployment"):
            if not nonempty(row.get(field)):
                errors.append(f"missing {field}: {key}")
        caps = row.get("capabilities", {})
        if not isinstance(caps, dict) or any(not nonempty(caps.get(k)) for k in CAPABILITIES):
            errors.append(f"missing capability assessment: {key}")
        for field in ("request_protocols", "endpoints", "limitations"):
            value = row.get(field)
            if not isinstance(value, list) or any(not nonempty(v) for v in value):
                errors.append(f"invalid {field}: {key}")
        protocols = row.get("request_protocols", [])
        if isinstance(protocols, list) and any(p not in PROTOCOLS for p in protocols if isinstance(p, str)):
            errors.append(f"unknown request protocol: {key}")
        refs = row.get("evidence_ids")
        if not isinstance(refs, list) or not refs or any(not isinstance(e, str) or e not in evidence for e in refs):
            errors.append(f"missing/dangling evidence: {key}")
            refs = []
        types = {evidence[e]["type"] for e in refs if "type" in evidence[e]}
        if conclusion == "资料不足" and not types & {"official_query", "official_docs", "official_sdk"}:
            errors.append(f"insufficient research without official query attempt: {key}")
        if conclusion == "官方确认" and not types & {"official_docs", "official_sdk"}:
            errors.append(f"official claim without official evidence: {key}")
        if conclusion in {"官方确认", "实测通过"} and (not protocols or not row.get("endpoints")):
            errors.append(f"supported protocol/endpoint missing: {key}")
        if conclusion == "实测通过":
            live = row.get("live_verification", {})
            if ("real_call" not in types or not isinstance(live, dict)
                    or any(not live.get(k) for k in ("http_status", "binding", "events", "final_state"))):
                errors.append(f"live claim without binding/events/final state: {key}")
    if seen != set(expected):
        errors.append(f"source row coverage mismatch: missing={len(set(expected)-seen)}, extra={len(seen-set(expected))}")
    return errors


def check_coverage(inventory, matrix):
    """Validate every source identity and all four cells; never imply live service success."""
    errors = []
    if not isinstance(matrix, dict) or matrix.get("schema_version") != 1:
        return ["invalid matrix schema_version"]
    hashes = {s: v["sha256"] for s, v in inventory["sources"].items()}
    if matrix.get("source_hashes") != hashes:
        errors.append("source_hashes conflict")
    for source, digest in hashes.items():
        path = Path(source)
        if not path.is_absolute():
            path = ROOT / path
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            errors.append(f"stale inventory source: {source}")
    try:
        date.fromisoformat(matrix.get("checked_at", ""))
    except (TypeError, ValueError):
        errors.append("invalid checked_at")
    current = build_inventory([Path(s) if Path(s).is_absolute() else ROOT / s for s in hashes])
    if current["records"] != inventory["records"]:
        errors.append("inventory records conflict with complete source catalogs")
    expected = {identity(r): r for r in inventory["records"]}
    rows = matrix.get("records")
    if not isinstance(rows, list):
        return errors + ["records must be an array"]
    seen = set()
    for row in rows:
        if not isinstance(row, dict):
            errors.append("invalid matrix record")
            continue
        key = identity(row)
        if not all(isinstance(p, str) for p in key):
            errors.append("invalid source identity")
            continue
        if key in seen or key not in expected:
            errors.append(f"duplicate or unexpected source row: {key}")
        seen.add(key)
        original = expected.get(key, {})
        for field, source_field in (("model_api_id", "model_api_id"),
                ("canonical_model_id", "canonical_model_id"),
                ("source_declarations", "effective_declarations"), ("capabilities", "capabilities")):
            if field not in row or row[field] != original.get(source_field):
                errors.append(f"{field} conflict: {key}")
        if row.get("evidence_type") != "offline_selector":
            errors.append(f"invalid offline evidence type: {key}")
        protocols = row.get("protocols")
        if not isinstance(protocols, dict) or set(protocols) != PROTOCOLS:
            errors.append(f"four protocol coverage mismatch: {key}")
            continue
        for source, attempts in protocols.items():
            if not isinstance(attempts, list) or not attempts:
                errors.append(f"missing protocol outcome: {key}/{source}")
                continue
            deployments = set()
            for attempt in attempts:
                if not isinstance(attempt, dict):
                    errors.append(f"invalid attempt: {key}/{source}")
                    continue
                deployment = attempt.get("deployment")
                if deployment in deployments:
                    errors.append(f"duplicate deployment: {key}/{source}")
                deployments.add(deployment)
                status = attempt.get("status")
                if status not in {"native", "conversion", "rejected"}:
                    errors.append(f"unknown status: {key}/{source}")
                elif status == "rejected":
                    reasons = attempt.get("reasons")
                    if not isinstance(reasons, list) or not reasons or any(not nonempty(r) for r in reasons):
                        errors.append(f"missing rejection reason: {key}/{source}")
                else:
                    target = attempt.get("target")
                    if (target not in PROTOCOLS or not nonempty(attempt.get("endpoint"))
                            or not nonempty(deployment)
                            or (status == "native") != (source == target)):
                        errors.append(f"invalid executable outcome: {key}/{source}")
                    candidates = row.get("candidate_endpoints") or []
                    if not any(e.get("protocol") == target and e.get("url") == attempt.get("endpoint")
                               and e.get("deployment") == deployment for e in candidates):
                        errors.append(f"outcome missing candidate evidence: {key}/{source}")
    if seen != set(expected) or len(rows) != len(expected):
        errors.append(f"source row coverage mismatch: missing={len(set(expected)-seen)}, extra={len(seen-set(expected))}")
    return errors


def write_json(path, value):
    if "records" in value:
        # One source row per line keeps the complete generated catalog reviewable.
        header = {k: v for k, v in value.items() if k != "records"}
        body = json.dumps(header, ensure_ascii=False, indent=2)[:-2]
        rows = ",\n".join("    " + json.dumps(r, ensure_ascii=False, separators=(",", ":"))
                          for r in value["records"])
        content = body + ',\n  "records": [\n' + rows + "\n  ]\n}\n"
    else:
        content = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
    path.write_text(content, encoding="utf-8")


def baseline_markdown(inventory, groups):
    rows = inventory["records"]
    missing = [p for p in inventory["providers"] if not p["declarations"].get("api")]
    overrides = Counter(k for r in rows for k in r["model_provider"])
    applicability = Counter(r["text_acp_applicability"] for r in rows)
    lines = ["# 全量目录核查基线", "", "核查日期：2026-11-11。证据类型：本地静态目录与源码审查；未进行官方查询或真实调用。", "",
             f"供应商身份 {inventory['provider_count']}；供应商来源记录 {inventory['provider_record_count']}；模型来源记录 {len(rows)}。",
             "来源身份为 `(source, provider_id, model_key)`，JSON Pointer 定位原始条目；API ID、canonical ID 独立保留。相同 canonical ID 不合并。", "",
             "## 来源与分组", "", "| 来源 | SHA-256 | 供应商 | 模型 |", "| --- | --- | ---: | ---: |"]
    for source, info in inventory["sources"].items():
        lines.append(f"| {source} | `{info['sha256']}` | {info['providers']} | {info['models']} |")
    lines += ["", f"新增供应商：{', '.join(groups['added_providers']) or '无'}；删除供应商：{', '.join(groups['removed_providers']) or '无'}。",
              f"源变化：{'有，见 research-groups.json source_changes' if groups['source_changes'] else '无，哈希和计数匹配初始目录基线'}。",
              "新增供应商按供应商数量最少的组分配，同量取较小组号；原有分配不变，空模型供应商也参与完整性核对。", "",
              "| 组 | 后续任务 | 供应商 | 模型来源记录 |", "| ---: | ---: | ---: | ---: |"]
    for group in groups["groups"]:
        lines.append(f"| {group['group']} | {group['task']} | {group['provider_count']} | {group['model_count']} |")
    lines += ["", "## 声明与缺口", "", f"模型 provider 覆盖 {sum(bool(r['model_provider']) for r in rows)} 条；字段计数：`{dict(sorted(overrides.items()))}`。",
              f"适用性初筛：`{dict(sorted(applicability.items()))}`。text 模态只表示候选；嵌入、实时、专用生成、停用状态和工具能力仍须逐条查询。",
              "effective_declarations 仅按字段应用模型覆盖，不推断 npm 与 adapter 等价、不推断原生端点、不宣称运行时已支持。", "",
              f"供应商未声明 API：{len(missing)} 家（必须查 SDK 默认值或部署参数，禁止猜测 URL）：", "",
              "| 供应商 | SDK/adapter 声明 | 文档种子（尚未查询） |", "| --- | --- | --- |"]
    for p in missing:
        lines.append(f"| {p['provider_id']} | {p['declarations'].get('npm', p['declarations'].get('adapter', '缺失'))} | {p['doc'] or '缺失'} |")
    lines += ["", "## 已审查源码与交付依赖", "",
              "- `crates/vw-shared/src/provider/models.rs`：Provider 保存 adapter、不保存 npm；ModelProviderInfo 仅保存 adapter，模型级 api/npm/shape 丢失。",
              "- `crates/vw-shared/src/provider/state.rs::from_models_dev_model`：模型 adapter 优先，API 使用供应商地址；缺省 adapter 会进入默认兼容路径。声明盘点不能据此证明上游兼容。",
              "- `crates/vw-provider-resolver/src/models.rs::parse_models_text/merge_bundled_shiju`：直接反序列化；只用 shiju.json 中的 shiju 整体替换，非 shiju 补充不会装载。本清单保留被遮蔽或未装载的来源行。",
              "- 开始时已有未提交依赖：`crates/vw-agent/src/model_relay/{mod.rs,server.rs,transport.rs,generation.rs}`、`crates/vw-agent-apps/src/aggregation/{mod.rs,model_ids_tests.rs}`、`crates/vw-agent/src/gateway/agent_apps_diagnostic_executors/{mod.rs,reachability.rs,model_check.rs,model_check/}`。这些文件属于共享传输、聚合和诊断工作；本任务没有修改、提交或验证它们，后续实现任务需核对实际交付。",
              "", "## 可重现命令与验证边界", "", "```bash",
              'python3 -m unittest discover -s scripts -p "test_audit_model_protocols.py"',
              "python3 scripts/audit_model_protocols.py inventory --api crates/vw-provider-resolver/assets/model/api.json --shiju crates/vw-provider-resolver/assets/model/shiju.json --output docs/dev/model-protocols",
              "python3 scripts/audit_model_protocols.py check-research --inventory docs/dev/model-protocols/catalog-inventory.json --report docs/dev/model-protocols/research/group-01.json --group 1", "```", "",
              "首次生成以计划目录基线为种子；后续生成默认复用输出 research-groups.json 的 baseline_groups/baseline_sources，可用 --baseline 显式指定种子。输入目录只读，输出可重复生成；源变化会记录增删和哈希差异。",
              "inventory 子命令核对来源行唯一性及各组互斥完整性，任何遗漏或冲突均非零退出。check-research 的成功仅证明合同和覆盖，不证明官方资料内容或服务实测正确。", "",
              "## 研究 JSON 合同 v1", "",
              "报告根字段：schema_version=1、group（整数）、source_hashes（全部来源路径→SHA-256）、providers（本组完整供应商 ID 列表）、evidence、records。",
              "evidence 每项：id（唯一）、type（official_docs/official_sdk/official_query/real_call/offline）、url（查询或来源 HTTP URL）、checked_at（ISO 日期）、excerpt（证据摘录或查询未果说明）。不保存密钥或敏感原始响应。",
              "records 每项必须逐条对应来源身份 source/provider_id/model_key，并保留 model_api_id、canonical_model_id（无值填 null）；不能只填 canonical ID 或一个供应商总论。",
              "每行还需 conclusion、reason、evidence_ids、request_protocols（chat/messages/responses/gemini）、endpoints（已确认端点字符串数组）、auth（认证方式）、deployment（地域/套餐/必要参数）、limitations（字符串数组）。",
              "capabilities 含 text/tools/thinking/media/streaming/structured_output 六个非空字符串评估；thinking 应涵盖关闭/强制/回放，streaming 应区分 JSON/SSE/终态；未知项明确写 unknown 并解释。",
              "conclusion 仅允许：官方确认、实测通过、明确不支持、资料不足、不适用于文本ACP、缺少凭据、实现缺口。资料不足也必须提供查询 URL、日期、摘录/未果说明及 reason；端点和协议未知可填空数组。",
              "官方确认需 official_docs 或 official_sdk 证据以及非空协议、端点；实测通过需 real_call 证据和 live_verification 的 http_status/binding/events/final_state。401/403、限流、超时和未知模型应在 reason 分别记录，不能据单次失败断言永久不支持。",
              "报告严格核对分组、全部源哈希、来源身份、API ID、canonical ID、重复行、漏行及证据引用；不访问网络，不代替官方或实时验收。", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    inv = commands.add_parser("inventory")
    for field in ("api", "shiju", "output"):
        inv.add_argument(f"--{field}", type=Path, required=True)
    inv.add_argument("--baseline", type=Path)
    check = commands.add_parser("check-research")
    check.add_argument("--inventory", type=Path, required=True)
    check.add_argument("--report", type=Path, required=True)
    check.add_argument("--group", type=int, required=True)
    coverage = commands.add_parser("check-coverage")
    coverage.add_argument("--inventory", type=Path, required=True)
    coverage.add_argument("--matrix", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "inventory":
            output_paths = [args.output / name for name in (
                "catalog-inventory.json", "research-groups.json", "catalog-baseline.md")]
            if {p.resolve() for p in (args.api, args.shiju)} & {p.resolve() for p in output_paths}:
                raise ValueError("output must not overwrite source catalogs")
            inventory = build_inventory([args.api, args.shiju])
            seed_path = args.baseline or (output_paths[1] if output_paths[1].exists() else INITIAL_BASELINE)
            groups = build_groups(inventory, load_json(seed_path))
            inventory["research_groups"] = groups["groups"]
            rows = inventory["records"]
            assigned = [p for g in groups["groups"] for p in g["providers"]]
            if (len({identity(r) for r in rows}) != len(rows)
                    or len(assigned) != len(set(assigned))
                    or set(assigned) != {p["provider_id"] for p in inventory["providers"]}
                    or sum(g["model_count"] for g in groups["groups"]) != len(rows)):
                raise ValueError("inventory/group completeness failure")
            args.output.mkdir(parents=True, exist_ok=True)
            write_json(output_paths[0], inventory)
            write_json(output_paths[1], groups)
            output_paths[2].write_text(baseline_markdown(inventory, groups), encoding="utf-8")
            print(f"OK providers={inventory['provider_count']} records={len(rows)} groups={len(groups['groups'])}")
        elif args.command == "check-coverage":
            inventory = load_json(args.inventory)
            matrix = load_json(args.matrix)
            errors = check_coverage(inventory, matrix)
            if errors:
                for error in errors:
                    print(error, file=sys.stderr)
                return 1
            print(f"OK records={len(matrix['records'])} protocol_cells={4 * len(matrix['records'])} missing=0 duplicate=0")
        else:
            inventory = load_json(args.inventory)
            report = load_json(args.report)
            errors = check_research(inventory, report, args.group)
            if errors:
                for error in errors:
                    print(error, file=sys.stderr)
                return 1
            print(f"OK group={args.group} providers={len(report['providers'])} records={len(report['records'])}")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
