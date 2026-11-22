"""Build the local-relay Cortex engine from a fixed supplier SEA."""
import hashlib
from pathlib import Path
import struct
import subprocess
import sys

SUPPLIER_SHA256 = 'f9753a0da7a733d4e782dfe633c7bd4fd5faca7b4960352d0aa464b62441178a'
SEA_OFFSET = 86687744


def rewrite_script(binary, sea_offset):
    # Node SEA v22 stores the script inline. This release has no startup snapshot
    # or code cache. Keep the serialized fields that follow the script intact.
    magic, flags, format = struct.unpack_from('<IIB', binary, sea_offset)
    if (magic, flags, format) != (0x0143DA20, 1, 1):
        raise RuntimeError('Unexpected Cortex Code SEA header')
    name_size, = struct.unpack_from('<Q', binary, sea_offset + 9)
    size_offset = sea_offset + 17 + name_size
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
            'let shijuMcp=new Set;function shijuTool(t){return t==="read"||t==="write"||t==="edit"||shijuMcp.has(t)}function YFp(){')
    # Only reviewed native tools and client tools registered by the launcher are visible.
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
    # Use Cortex's native session registry and the native read-based catalog
    # formatter. Chat Completions otherwise omits the discovered skill metadata.
    replace('for(let i of M9i)e.push([i,"session"]);',
            'for(let i of M9i)e.push([i,"session"]);'
            'if(shijuLocal()&&process.env.SHIJU_CORTEX_SKILLS)'
            'e.push([process.env.SHIJU_CORTEX_SKILLS,"session"]);')
    replace('tools:A1g(l.tools||l["allowed-tools"])}',
            'tools:A1g(l.tools||l["allowed-tools"]),'
            'disableModelInvocation:l["disable-model-invocation"]===true}')
    replace('case"name":e.name=r;break;case"description":if(r===',
            'case"name":e.name=r;break;'
            'case"disable-model-invocation":e.disableModelInvocation=/^true(?:\\s+#.*)?$/i.test(r);break;'
            'case"description":if(r===')
    replace('location:e,instructions:p,basePath:t,tools:f',
            'location:e,instructions:p,basePath:t,tools:f,'
            'disableModelInvocation:s.disableModelInvocation')
    replace('toolSnippets:e,appendSystemPrompt:t.appendSystemPrompt})',
            'toolSnippets:e,appendSystemPrompt:t.appendSystemPrompt,'
            'skills:shijuLocal()?Tg().list().map(s=>({name:s.name,description:s.description,'
            'filePath:s.basePath+"/SKILL.md",disableModelInvocation:s.disableModelInvocation})):void 0})')
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
    return size_offset, start, size, script


def patch(source, destination):
    binary = bytearray(Path(source).read_bytes())
    if hashlib.sha256(binary).hexdigest() != SUPPLIER_SHA256:
        raise RuntimeError('Cortex Code supplier executable checksum mismatch')
    size_offset, start, size, script = rewrite_script(binary, SEA_OFFSET)
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


# The SHA-256 check binds these offsets to the exact supplier ELF. The SEA is the
# last note in the last, read-only PT_LOAD, with zero padding before the section table.
LINUX_ELF = {
    'x86_64': ('42cbc28b4ceef8e8c4e897e0db447b6d47cb32a625b5434830de4b23824e4313',
               62, 0x7720000),
    'aarch64': ('a894b18f42af7b4d0f74f0d790b0850fca4bd9902f38ae9020008297a706bb41',
                183, 0x74b0000),
}


def patch_linux(source, destination, architecture):
    digest, machine, segment = LINUX_ELF[architecture]
    binary = bytearray(Path(source).read_bytes())
    if hashlib.sha256(binary).hexdigest() != digest:
        raise RuntimeError('Cortex Code Linux supplier executable checksum mismatch')
    phoff, shoff = struct.unpack_from('<QQ', binary, 32)
    phsize, phcount, shsize, shcount = struct.unpack_from('<4H', binary, 54)
    if (binary[:6] != b'\x7fELF\x02\x01'
            or struct.unpack_from('<H', binary, 18)[0] != machine
            or (phsize, shsize) != (56, 64)):
        raise RuntimeError('Unexpected Cortex Code ELF layout')
    note, sea = segment + 0x16c, segment + 0x188
    namesize, blob_size, note_type = struct.unpack_from('<III', binary, note)
    if (namesize, note_type, binary[note + 12:note + 12 + namesize]) != (14, 0, b'NODE_SEA_BLOB\0'):
        raise RuntimeError('Unexpected Cortex Code ELF SEA note')
    note_end = (sea + blob_size + 3) & ~3
    headers = {}
    for header in range(phoff, phoff + phcount * phsize, phsize):
        kind, flags, offset, _, _, filesz, memsz, _ = struct.unpack_from('<II6Q', binary, header)
        if offset == segment and flags == 4 and filesz == memsz and kind in (1, 4):
            if kind in headers:
                raise RuntimeError('Duplicate Cortex Code ELF segment')
            headers[kind] = (header, offset + filesz)
    if (headers.keys() != {1, 4} or headers[4][1] != note_end or headers[1][1] != shoff):
        raise RuntimeError('Unexpected Cortex Code ELF SEA segments')
    sections = [header for header in range(shoff, shoff + shcount * shsize, shsize)
                if struct.unpack_from('<I', binary, header + 4)[0] == 7
                and struct.unpack_from('<QQ', binary, header + 24) == (segment + 0x100, 0x1dfcbb8)]
    if len(sections) != 1:
        raise RuntimeError('Unexpected Cortex Code ELF SEA section')

    size_offset, start, size, script = rewrite_script(binary, sea)
    trailing = bytes(binary[start + size:sea + blob_size])
    new_blob_size = start - sea + len(script) + len(trailing)
    new_note_end = (sea + new_blob_size + 3) & ~3
    growth = new_note_end - note_end
    if growth < 0 or new_note_end > shoff or any(binary[note_end:new_note_end]):
        raise RuntimeError('Cortex Code local composition exceeds ELF SEA alignment gap')

    struct.pack_into('<I', binary, note + 4, new_blob_size)
    struct.pack_into('<QQ', binary, headers[4][0] + 32, new_note_end - segment, new_note_end - segment)
    struct.pack_into('<Q', binary, sections[0] + 32, 0x1dfcbb8 + growth)
    struct.pack_into('<Q', binary, size_offset, len(script))
    binary[start:new_note_end] = script + trailing + bytes(new_note_end - sea - new_blob_size)
    destination = Path(destination)
    destination.write_bytes(binary)
    destination.chmod(0o700)


if __name__ == '__main__':
    args = sys.argv[1:]
    if len(args) == 4 and args[0] == '--linux' and args[1] in LINUX_ELF:
        patch_linux(args[2], args[3], args[1])
    elif len(args) == 2:
        patch(*args)
    else:
        sys.exit('Usage: apply.py [--linux x86_64|aarch64] SOURCE DESTINATION')
