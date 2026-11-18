"""Build a separate local-relay Cortex runtime from the reviewed Mach-O SEA."""
import hashlib
from pathlib import Path
import struct
import subprocess
import sys

SUPPLIER_SHA256 = 'f9753a0da7a733d4e782dfe633c7bd4fd5faca7b4960352d0aa464b62441178a'
SEA_OFFSET = 86687744


def patch(source, destination):
    binary = bytearray(Path(source).read_bytes())
    if hashlib.sha256(binary).hexdigest() != SUPPLIER_SHA256:
        raise RuntimeError('Cortex Code supplier executable checksum mismatch')
    # Node SEA v22 stores the script inline. This release has no startup snapshot
    # or code cache. Expand the script into the existing Mach-O alignment gap.
    magic, flags, format = struct.unpack_from('<IIB', binary, SEA_OFFSET)
    if (magic, flags, format) != (0x0143DA20, 1, 1):
        raise RuntimeError('Unexpected Cortex Code SEA header')
    name_size, = struct.unpack_from('<Q', binary, SEA_OFFSET + 9)
    size_offset = SEA_OFFSET + 17 + name_size
    size, = struct.unpack_from('<Q', binary, size_offset)
    start = size_offset + 8
    script = bytes(binary[start:start + size])

    def replace(before, after):
        nonlocal script
        before, after = before.encode(), after.encode()
        if script.count(before) != 1:
            raise RuntimeError(f'Cortex Code patch anchor changed: {before[:70]!r}')
        script = script.replace(before, after)

    replace('function YFp(){',
            'function shijuLocal(){if(process.env.SHIJU_CORTEX_LOCAL!=="1")return false;'
            'let t=YFp();if(!t||!/^http:\\/\\/127\\.0\\.0\\.1:[0-9]+\\/v1$/.test(t.endpoint)'
            '||!process.env.CORTEX_AGENT_MODEL)throw new Error("Invalid local Cortex model binding");return true}'
            'let shijuMcp=new Set;function shijuTool(t){return shijuMcp.size?shijuMcp.has(t):t==="read"||t==="write"||t==="edit"}function YFp(){')
    # Only names registered by the launcher are visible to managed inference.
    replace('this.registeredClientTools.push(i),fEp(i)',
            'this.registeredClientTools.push(i),fEp(i),'
            'shijuLocal()&&i.startsWith("shiju_mcp_")&&shijuMcp.add(i)')
    replace('e.unregister(n),mEp(n),Qx.info(`Unregistered client tool:',
            'e.unregister(n),mEp(n),shijuMcp.delete(n),Qx.info(`Unregistered client tool:')
    # Compose native inference without the hosted connection in local-only mode.
    replace('vCs=class extends TCs{async initialize(){',
            'vCs=class extends TCs{async initialize(){if(shijuLocal())return;')
    replace('if(!this.connectionInfo){zi.error("No Snowflake connection available")',
            'if(!this.connectionInfo&&!shijuLocal()){zi.error("No Snowflake connection available")')
    replace('getModel(){return this._model??"auto"}',
            'getModel(){return shijuLocal()?process.env.CORTEX_AGENT_MODEL:this._model??"auto"}')
    replace('async getCachedModels(e){if(this.modelsFetched)',
            'async getCachedModels(e){if(shijuLocal()){let id=process.env.CORTEX_AGENT_MODEL;'
            'return[{id,name:id}]}if(this.modelsFetched)')
    replace('async setModel(e,n){let o=this.sessionManager.getAgentService(e);',
            'async setModel(e,n){if(shijuLocal()&&n!==process.env.CORTEX_AGENT_MODEL)'
            'throw new Error("Cortex model is bound by ShiJu");let o=this.sessionManager.getAgentService(e);')
    replace('let o=fY(),s=n.getPlanMode()?', 'let o=!shijuLocal()&&fY(),s=n.getPlanMode()?')
    replace('setDangerouslyAllowAll(e){this.permissionManager&&',
            'setDangerouslyAllowAll(e){if(shijuLocal()&&e)throw new Error("Local Cortex requires tool approval");this.permissionManager&&')
    # The native terminal probe would start osascript. Managed contexts forbid
    # all child processes and use ACP for permission prompts.
    replace('function Ej_(){if(', 'function Ej_(){if(shijuLocal()||')
    replace('a=this.resolveActiveToolDefinitions();zi.info(`BENCHMARK: Making ${e.id}',
            'a=this.resolveActiveToolDefinitions();if(shijuLocal())a=a.filter(t=>shijuTool(t.name));'
            'zi.info(`BENCHMARK: Making ${e.id}')
    replace('let l={...r,ttfr_ms:r.ttfr_ms??u,ttft_ms:r.ttft_ms??u};',
            'if(shijuLocal()&&r.toolUses.some(t=>!shijuTool(t.name)))'
            'throw new Error("Cortex received an unavailable local tool");'
            'let l={...r,ttfr_ms:r.ttfr_ms??u,ttft_ms:r.ttft_ms??u};')
    replace('!n.titleGenerationAttempted&&!e8e(i)', '!shijuLocal()&&!n.titleGenerationAttempted&&!e8e(i)')
    replace('r instanceof SWe)throw r;return await this.connection.sessionUpdate',
            'r instanceof SWe||shijuLocal())throw r;return await this.connection.sessionUpdate')
    segment = binary.index(b'NODE_SEA\0', 32, 4096) - 8
    section = segment + 72
    if (struct.unpack_from('<II', binary, segment) != (0x19, 152)
            or binary[section:section + 16] != b'__NODE_SEA_BLOB\0'
            or struct.unpack_from('<Q', binary, segment + 40)[0] != SEA_OFFSET):
        raise RuntimeError('Unexpected Cortex Code Mach-O SEA layout')
    blob_size, = struct.unpack_from('<Q', binary, segment + 48)
    next_offset, = struct.unpack_from('<Q', binary, segment + 152 + 40)
    trailing = bytes(binary[start + size:SEA_OFFSET + blob_size])
    new_blob_size = start - SEA_OFFSET + len(script) + len(trailing)
    virtual_size, = struct.unpack_from('<Q', binary, segment + 32)
    if (new_blob_size > virtual_size or SEA_OFFSET + new_blob_size > next_offset
            or any(binary[SEA_OFFSET + blob_size:SEA_OFFSET + new_blob_size])):
        raise RuntimeError('Cortex Code local composition exceeds the SEA alignment gap')
    struct.pack_into('<Q', binary, segment + 48, new_blob_size)
    struct.pack_into('<Q', binary, section + 40, new_blob_size)
    struct.pack_into('<Q', binary, size_offset, len(script))
    binary[start:SEA_OFFSET + new_blob_size] = script + trailing
    destination = Path(destination)
    destination.write_bytes(binary)
    destination.chmod(0o700)
    subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                    '--identifier', 'shiju.cortex.local', str(destination)], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)


if __name__ == '__main__':
    patch(sys.argv[1], sys.argv[2])
