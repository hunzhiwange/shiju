"""Reproducible static audit. No Cargo, tests, cfg evaluation or source mutation."""
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PLAN = Path(__file__).resolve().parent.parent
OUT = PLAN / 'evidence'

def dump(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')

baseline = json.loads((PLAN / 'baseline.json').read_text())
scope = json.loads((PLAN / 'scope.json').read_text())
owners = {p: row[0] for row in scope['rows'] for p in row[5]}
owners.update({p: 59 for p in scope['residue']})
counts = Counter(p for row in scope['rows'] for p in row[5])
counts.update(scope['residue'])
assert set(counts) == set(baseline)
assert all(count == 1 for count in counts.values())

def inventory(path):
    text = path.read_text()
    lines = text.splitlines()
    # Lexical candidates, not compiler-resolved symbols or production accounting.
    return dict(path=str(path.relative_to(ROOT)), lines=len(lines),
                symbols=[dict(line=i, declaration=line.strip())
                         for i, line in enumerate(lines, 1)
                         if re.match(r'\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:fn|struct|enum|trait|impl)\b', line)],
                cfg=[dict(line=i, attribute=line.strip())
                     for i, line in enumerate(lines, 1) if '#[cfg' in line],
                exports=[dict(line=i, declaration=line.strip())
                         for i, line in enumerate(lines, 1)
                         if re.match(r'\s*pub(?:\([^)]*\))?\s+use\b', line)])

host = {str(p.relative_to(ROOT)): inventory(p)
        for p in sorted((ROOT / 'crates/vw-agent/src').rglob('*.rs'))}
original = [dict(path=p, owner=owners[p], before=n,
                 after=host.get(p, {}).get('lines', 0),
                 disposition='retained_requires_review' if p in host else 'removed_requires_destination_mapping',
                 evidence=f'evidence/task-{owners[p]}.md')
            for p, n in sorted(baseline.items())]
alias_map = {row['source']: row for row in json.loads((OUT/'task-59-aliases.json').read_text())}
orphans = {row['source'] for row in json.loads((OUT/'task-59-orphans.json').read_text())}
for row in original:
    if row['path'] in alias_map:
        row['disposition'] = 'direct_domain_alias'
        row['destination'] = alias_map[row['path']]['domain']
    elif row['path'] in orphans:
        row['disposition'] = 'removed_unregistered_forwarder'
        row['destination'] = 'agent_memory existing public modules'
remaining = [dict(**row, symbols=host[row['path']]['symbols'])
             for row in original if row['after']]
totals = Counter()
for row in remaining:
    totals[row['owner']] += row['after']
new_host = [v for p, v in host.items() if p not in baseline]
total = sum(v['lines'] for v in host.values())
domain = [inventory(p) for p in sorted((ROOT / 'crates/libs').rglob('*.rs'))
          if 'host_migration_' in str(p)]
reverse = []
for path in sorted((ROOT / 'crates/libs').rglob('*')):
    if not path.is_file() or (path.suffix != '.rs' and path.name != 'Cargo.toml'):
        continue
    for i, line in enumerate(path.read_text().splitlines(), 1):
        if (path.name == 'Cargo.toml' and re.search(r'\bvw-agent\s*=', line)) or (
                path.suffix == '.rs' and re.search(r'(?:path\s*=|include(?:_str|_bytes)?!).*vw-agent(?:/|\\)', line)):
            reverse.append(dict(path=str(path.relative_to(ROOT)), line=i, source=line.strip()))
dump('after-files.json', dict(method='pathlib read_text splitlines; cfg union; physical files',
                             original=original, host=list(host.values()),
                             new_host=new_host, migration_named_domain_files=domain,
                             domain_inventory_limit='Only host_migration named files; not a complete migration destination map'))
dump('remaining-host.json', dict(status='physical_gate_passed' if total <= 35000 else 'physical_gate_failed', host_lines=total, limit=35000,
                                excess=max(0, total - 35000), baseline_lines=sum(baseline.values()),
                                net_physical_reduction=sum(baseline.values()) - total,
                                owner_totals=dict(sorted(totals.items())), files=remaining,
                                new_host=new_host, reverse_dependency_candidates=reverse,
                                required_action='Original owners must split domain assertions and logic from listed symbols; no entire file approved as a boundary. Restore original task sessions before final closure.',
                                mapping_status='Full old-test and production-call mappings not independently verified; do not claim coverage preservation.'))
evidence = []
for n in range(1, 59):
    path = OUT / f'task-{n}.md'
    text = path.read_text() if path.exists() else ''
    evidence.append(dict(task=n, exists=path.exists(), lines=len(text.splitlines()),
                         unresolved_markers=[dict(line=i, text=line.strip())
                                             for i, line in enumerate(text.splitlines(), 1)
                                             if re.search(r'阻塞|未闭合|未迁|unresolved|blocked|remaining', line, re.I)]))
dump('task-59-evidence-index.json', evidence)
print(json.dumps(dict(original_files=len(original), unique_ownership=True,
                      host_files=len(host), host_lines=total, excess=total-35000,
                      residue_lines=totals[59], new_host_files=len(new_host),
                      reverse_candidates=reverse), ensure_ascii=False, indent=2))
