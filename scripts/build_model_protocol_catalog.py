#!/usr/bin/env python3
"""Project dated research into conservative, exact-model runtime protocol facts."""
import argparse
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORTS = ROOT / 'docs/dev/model-protocols/research'
ASSET = ROOT / 'crates/vw-provider-resolver/assets/model/protocols.json'
# SDK defaults verified by the reports; project/region/local-runtime SDKs stay unset.
DEFAULTS = {
    'anthropic': 'https://api.anthropic.com/v1',
    'cerebras': 'https://api.cerebras.ai/v1',
    'deepinfra': 'https://api.deepinfra.com/v1/openai',
    'google': 'https://generativelanguage.googleapis.com/v1beta',
    'groq': 'https://api.groq.com/openai/v1',
    'mistral': 'https://api.mistral.ai/v1',
    'openai': 'https://api.openai.com/v1',
    'salad-cloud': 'https://ai.salad.cloud/v1',
    'togetherai': 'https://api.together.ai/v1',
    'venice': 'https://api.venice.ai/api/v1',
    'xai': 'https://api.x.ai/v1',
}


def authentication(provider, protocol, text):
    """Only publish known HTTP header contracts; CLI token names prove no header."""
    if provider == 'google' and protocol == 'gemini':
        return {'kind': 'api_key_header', 'name': 'x-goog-api-key'}
    if protocol == 'messages':
        if 'x-api-key' in text:
            return {'kind': 'api_key_header', 'name': 'x-api-key'}
        if provider.startswith('alibaba'):
            # Report says Anthropic-compatible authentication, but not an exact header.
            return None
    if 'Bearer' in text:
        return {'kind': 'bearer'}
    return None


def operation(protocol, url):
    # Exclude dedicated SDK endpoints even when the report names their surface "chat".
    return url.startswith('https://') and (
        (protocol == 'chat' and url.endswith('/chat/completions'))
        or (protocol == 'messages' and url.endswith('/messages'))
        or (protocol == 'responses' and url.endswith('/responses'))
        or (protocol == 'gemini' and ':generateContent' in url)
    )


def build():
    api = json.loads((ROOT / 'crates/vw-provider-resolver/assets/model/api.json').read_text())
    shiju = json.loads((ROOT / 'crates/vw-provider-resolver/assets/model/shiju.json').read_text())
    raw = {**api, **shiju}
    providers = {}
    reviews = {}
    grouped = {}
    excluded = collections.Counter()
    for number in range(1, 7):
        path = REPORTS / f'group-{number:02}.json'
        report = json.loads(path.read_text())
        for source, expected in report['source_hashes'].items():
            if hashlib.sha256((ROOT / source).read_bytes()).hexdigest() != expected:
                raise ValueError(f'research source drift: {source}')
        evidence = {item['id']: item for item in report['evidence']}
        findings = report.get('provider_assessments', report.get('provider_findings', report.get('provider_reviews', [])))
        if isinstance(findings, dict):
            findings = [dict(value, provider_id=key) for key, value in findings.items()]
        findings = {item['provider_id']: item for item in findings}
        for provider_id in report['providers']:
            if provider_id in api and not api[provider_id].get('api'):
                reviews[provider_id] = {
                    'npm': api[provider_id]['npm'],
                    'default_api': DEFAULTS.get(provider_id),
                    'report': str(path.relative_to(ROOT)),
                    'assessment': findings.get(provider_id, {}),
                    'implementation_boundary': (
                        'Dedicated SDK/auth/project/region/local deployment requires explicit binding; '
                        'unknown SDK identity remains unchanged. Protocol selection task 11, '
                        'transport wiring task 12, coverage task 20 must track unresolved rows.'
                    ),
                }
        for row in report['records']:
            if row['conclusion'] != '官方确认':
                excluded[row['conclusion']] += 1
                continue
            provider_id = row['provider_id']
            model = raw[provider_id]['models'][row['model_key']]
            override = model.get('provider') or {}
            base = override.get('api', raw[provider_id].get('api')) or DEFAULTS.get(provider_id)
            if not base:
                excluded['missing_deployment_base'] += 1
                continue
            for protocol in row['request_protocols']:
                auth = authentication(provider_id, protocol, row['auth'])
                urls = [url for url in row['endpoints'] if operation(protocol, url)]
                if auth is None or len(urls) != 1:
                    excluded['unresolved_auth_or_operation'] += 1
                    continue
                refs = [{
                    'kind': 'official', 'source': evidence[ref]['url'],
                    'reference': f'{path.relative_to(ROOT)}#records/{row["model_key"]};evidence/{ref}',
                    'checked_at': report['checked_at'],
                } for ref in row['evidence_ids'] if ref in evidence]
                if not refs:
                    raise ValueError(f'missing evidence: {provider_id}/{row["model_key"]}')
                key = (provider_id, base.rstrip('/'), protocol, urls[0], json.dumps(auth, sort_keys=True))
                if key not in grouped:
                    grouped[key] = {
                        'protocol': protocol, 'url': urls[0], 'deployment': provider_id,
                        'authentication': auth, 'models': {'kind': 'models', 'ids': []},
                        'evidence': [],
                    }
                entry = grouped[key]
                entry['models']['ids'].append(row['model_api_id'])
                # Keep one per evidence ID, avoiding a separate copy per model in the asset.
                for ref in refs:
                    ref['reference'] = ref['reference'].split('#records/')[0] + '#evidence/' + ref['reference'].split(';evidence/')[1]
                    if ref not in entry['evidence']:
                        entry['evidence'].append(ref)
    for (provider_id, base, _, _, _), endpoint in sorted(grouped.items()):
        endpoint['models']['ids'] = sorted(set(endpoint['models']['ids']))
        provider = providers.setdefault(provider_id, {'default_api': DEFAULTS.get(provider_id), 'deployments': {}})
        provider['deployments'].setdefault(base, {'protocols': []})['protocols'].append(endpoint)
    for provider_id, base in DEFAULTS.items():
        providers.setdefault(provider_id, {'default_api': base, 'deployments': {}})
    return {
        'schema_version': 1, 'checked_at': '2026-11-11',
        'policy': 'Only officially confirmed exact-model rows with an unambiguous HTTP operation and auth header are published. Missing candidates are unknown. Source api.json remains unchanged.',
        'providers': providers,
        'sdk_default_reviews': dict(sorted(reviews.items())),
        'excluded_reasons': dict(sorted(excluded.items())),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = build()
    text = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if ASSET.read_text() != text:
            raise SystemExit('protocol catalog differs from research projection')
    else:
        ASSET.write_text(text)
    endpoints = [entry for provider in result['providers'].values() for entries in provider['deployments'].values() for entry in entries['protocols']]
    print(f"providers={len(result['providers'])} endpoints={len(endpoints)} model_bindings={sum(len(entry['models']['ids']) for entry in endpoints)} sdk_reviews={len(result['sdk_default_reviews'])}")


if __name__ == '__main__':
    main()
