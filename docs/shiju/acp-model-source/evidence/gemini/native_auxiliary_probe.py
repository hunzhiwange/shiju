#!/usr/bin/env python3
"""Supplemental real bundled-core requests, separate from full ACP acceptance."""
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "scripts"))
from acp_model_source_fixture import ModelFixture

module = ROOT / "target/acp-gemini-runtime/node_modules/@google/gemini-cli/bundle/chunk-RZ3ZVTZB.js"
program = r'''
const {Config,createContentGenerator}=await import(process.env.PROBE_MODULE);
process.argv.push('--shiju-model-source');
const config=new Config({sessionId:'offline-helper-probe',targetDir:process.cwd(),cwd:process.cwd(),model:process.env.GEMINI_MODEL,debugMode:false,telemetry:{enabled:false}});
const generator=await createContentGenerator({authType:'gateway',apiKey:process.env.GEMINI_API_KEY,baseUrl:process.env.GOOGLE_GEMINI_BASE_URL},config,'offline-helper-probe');
for(const alias of ['classifier','summarizer-default','chat-compression-default','agent-history-provider-summarizer']) {
 const resolved=config.modelConfigService.getResolvedConfig({model:alias});
 if(resolved.model!==process.env.GEMINI_MODEL)throw new Error('auxiliary escaped selected model');
 await generator.generateContent({model:resolved.model,contents:[{role:'user',parts:[{text:'offline helper routing probe'}]}],config:resolved.generateContentConfig},'helper-probe','utility');
}
for (const signature of ['skip_thought_signature_validator','opaque-provider-signature']) {
 await generator.generateContent({model:process.env.GEMINI_MODEL,contents:[
  {role:'user',parts:[{text:'signature transport probe'}]},
  {role:'model',parts:[{functionCall:{id:'probe-call',name:'read_file',args:{}},thoughtSignature:signature}]},
  {role:'user',parts:[{functionResponse:{id:'probe-call',name:'read_file',response:{output:'probe-only'}}}]}
 ]},'signature-probe','utility');
}
await config.dispose();
'''
with tempfile.TemporaryDirectory(prefix="gemini-helper-probe-") as root, ModelFixture() as fixture:
    signatures = []
    handler = fixture.server.RequestHandlerClass
    original_post = handler.do_POST

    def inspect_signature(request):
        raw = request.rfile.read(int(request.headers["Content-Length"]))
        request.rfile = io.BytesIO(raw)
        body = json.loads(raw)
        signatures.extend(part["thoughtSignature"] for content in body.get("contents", [])
                          for part in content.get("parts", []) if "thoughtSignature" in part)
        return original_post(request)

    handler.do_POST = inspect_signature
    env = {key: os.environ[key] for key in ("PATH", "HOME", "TMPDIR") if key in os.environ}
    env.update(PROBE_MODULE=module.as_uri(), GEMINI_CLI_HOME=root, GEMINI_MODEL=fixture.models[0],
               GEMINI_API_KEY=fixture.key, GOOGLE_GEMINI_BASE_URL=fixture.url,
               SHIJU_GEMINI_HISTORY_ROOT=str(Path(root) / "history"))
    result = subprocess.run(["node", "--input-type=module", "-e", program], cwd=root, env=env,
                            capture_output=True, timeout=40)
    evidence = fixture.snapshot()
    # Do not store raw native diagnostics, which may include request context.
    passed = result.returncode == 0 and evidence["errors"] == 0 and len(evidence["requests"]) == 6 and signatures == ["opaque-provider-signature"]
    print(json.dumps({"evidence_kind":"real-bundled-core-supplemental", "exit_code":result.returncode,
        "passed":passed, "synthetic_signature_removed": "skip_thought_signature_validator" not in signatures,
        "opaque_signature_preserved": signatures == ["opaque-provider-signature"], "aliases":["classifier","summarizer-default","chat-compression-default","agent-history-provider-summarizer"],
        "requests":evidence},indent=2))
    if not passed:
        # Only a closed source location/category, never native diagnostics.
        print("Native auxiliary probe failed", file=sys.stderr)
    raise SystemExit(0 if passed else 1)
