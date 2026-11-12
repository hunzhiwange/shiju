"""Select only the fixed supplier's local ACP framework; never alter cloud auth."""
import hashlib
import json
from pathlib import Path
import shutil
import sys

root = Path(sys.argv[1])
assets = Path(__file__).parent
contract = json.loads((assets / 'patch.json').read_text())
entry = root / 'bundle/qodercli.js'
if hashlib.sha256(entry.read_bytes()).hexdigest() != contract['original_sha256']:
    raise RuntimeError('Qoder supplier bundle checksum mismatch')
source = entry.read_text()
anchor = 'case"acp":{let{QoderAcpAppFramework:t}=await Promise.resolve().then(()=>(XTa(),KTa));return new t(A,e,i)}'
# P7e/ahe are Qoder's own local ACP framework/server, using the same e5/O1
# session host, agent loop, tools and permission controller. Cloud Tk/FVt are
# never constructed in this branch. The original branch remains unchanged.
replacement = 'case"acp":{let{QoderAcpAppFramework:t}=await Promise.resolve().then(()=>(XTa(),KTa));if(shijuQoder.active){shijuQoder.validate();return new(class extends P7e{createModelProviders(){return SqA([JSON.stringify(shijuQoder.settings())],this.modelTransportManager.get(),this.settingsService,"user")}createAcpServer(A){return new(class extends ahe{async ensureAuthenticated(){shijuQoder.validate()}async ensureAuthReady(){shijuQoder.validate()}async prepareInference(A){shijuQoder.validate();if(A.config.getModel()!=="shiju/"+shijuQoder.validate().model)throw Error("Qoder session model changed")}})(A)}})(A,e,i)}return new t(A,e,i)}'
if source.count(anchor) != 1:
    raise RuntimeError('Qoder local ACP factory anchor mismatch')
source = source.replace(anchor, replacement)
newline = source.index('\n') + 1
source = source[:newline] + 'import * as shijuQoder from "./shiju-session-model.mjs";\n' + source[newline:]
entry.write_text(source)
shutil.copy2(assets / 'session-model.mjs', root / 'bundle/shiju-session-model.mjs')
