"""Build the pinned Droid 1.9.0 ACP executable for the selected target."""

import argparse
import hashlib
import json
import mmap
import os
from pathlib import Path
import shutil
import struct
import subprocess
from tempfile import TemporaryDirectory

SUPPLIER_SHA256 = "b62ce8de0ed7ffaada1abb2a017fe182021003694e85fe7f3b1136ad43628a5d"
GRAPH_DATA_OFFSET = 88_227_848
MODULE_RECORD_OFFSET = 311_196_249
MODULE_NAME = b"/$bunfs/root/chunk-b32e823k.js"
ORIGINAL = (
    b"function F(e,o){let t=[],s=[],n=new Map;for(let r of e)if(r.deferred)"
    b"if(o.has(r.spec.name))n.set(r.spec.name,r.spec);else s.push(r);else t.push(r.spec);"
    b"let i=[];for(let r of o){let a=n.get(r);if(a)i.push(a)}"
    b"return{exposed:[...t,...i],core:t,loadedDeferred:i,hidden:s}}"
)
EAGER_MCP = (
    b"function F(e,o){let t=[],s=[],n=new Map;for(let r of e)"
    b"r.deferred&&!XQ(r.spec.name)?o.has(r.spec.name)?n.set(r.spec.name,r.spec):s.push(r):t.push(r.spec);"
    b"let i=[];for(let r of o)n.has(r)&&i.push(n.get(r));"
    b"return{exposed:[...t,...i],core:t,loadedDeferred:i,hidden:s}}"
)

REMINDER_ANCHOR = b"h=[...P(i,n),...r],c=z(e,t);return"
REMINDER_EAGER = (
    b"h=[...P(i,n),...r],c=z(e.filter(q=>!XQ(q.spec.name)),t)"
    b"+'\\n\\nACP session MCP tools are already loaded. The following JSON list is the current "
    b"tool inventory: '+JSON.stringify(h.filter(q=>XQ(q.name)).map(q=>q.name))"
    b"+'\\nIf the user limits the task to one session MCP service, use only its tools. "
    b"Do not use ToolSearch or other tools to check a named MCP service or find additional tools. "
    b"If the requested service is absent from this list, return the fallback requested by the user. "
    b"When the user enumerates tools for a read-only investigation, use those tools only. "
    b"Keep all native permission requirements.';return"
)


LINUX_REMINDER_EAGER = (
    b"h=[...P(i,n),...r],c=z(e.filter(q=>!XQ(q.spec.name)),t)"
    b"+'\\n\\nACP session MCP tools are already loaded: '"
    b"+JSON.stringify(h.filter(q=>XQ(q.name)).map(q=>q.name));return"
)


def patch_source(binary, graph, module, module_name, bytecode, reminder):
    name, name_size, start, size = struct.unpack_from("<4I", binary, module)
    name += graph
    start += graph
    source_text = binary[start:start + size]
    if (
        binary[name:name + name_size] != module_name
        or source_text.count(ORIGINAL) != 1
        or source_text.count(REMINDER_ANCHOR) != 1
        or struct.unpack_from("<4I", binary, module + 24) != bytecode
    ):
        raise ValueError("Droid standalone tool projection layout changed")

    patched = source_text.replace(ORIGINAL, EAGER_MCP).replace(REMINDER_ANCHOR, reminder)
    cache_offset, cache_size, _, _ = bytecode
    if len(patched) + 1 > cache_size or cache_offset + cache_size > start - graph:
        raise ValueError("Droid patched source exceeds its module cache")
    # Store this chunk's source in its own unused bytecode space.
    cache_start = graph + cache_offset
    binary[cache_start:cache_start + len(patched) + 1] = patched + b"\0"
    struct.pack_into("<2I", binary, module + 8, cache_offset, len(patched))
    struct.pack_into("<4I", binary, module + 24, 0, 0, 0, 0)


def apply(source: Path, destination: Path, linux_arch=None) -> None:
    target = None
    if linux_arch is not None:
        manifest = json.loads(Path(__file__).with_name("linux.json").read_text())
        target = manifest["targets"][linux_arch]
    with TemporaryDirectory(prefix=".droid-patch-", dir=destination.parent) as temporary:
        executable = Path(temporary) / "droid"
        shutil.copyfile(source, executable)
        with executable.open("r+b") as stream, mmap.mmap(stream.fileno(), 0) as binary:
            digest = hashlib.sha256(binary).hexdigest()
            expected = SUPPLIER_SHA256 if target is None else target["source_sha256"]
            if digest != expected:
                raise ValueError("Droid supplier executable checksum mismatch")
            if target is None:
                patch_source(binary, GRAPH_DATA_OFFSET, MODULE_RECORD_OFFSET, MODULE_NAME,
                             (38_098_424, 22_824, 38_121_248, 0), REMINDER_EAGER)
            else:
                patch_source(binary, target["graph_offset"], target["module_offset"],
                             target["module_name"].encode(), tuple(target["bytecode"]),
                             LINUX_REMINDER_EAGER)
                if hashlib.sha256(binary).hexdigest() != target["patched_sha256"]:
                    raise ValueError("Droid patched executable checksum mismatch")

        executable.chmod(0o700)
        if target is None:
            subprocess.run(
                [
                    "/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none",
                    "--identifier", "shiju.droid.acp-diagnostic", str(executable),
                ],
                check=True,
            )
            subprocess.run(["/usr/bin/codesign", "--verify", str(executable)], check=True)
        os.link(executable, destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path, help="A new path in an existing private directory")
    parser.add_argument("--linux-arch", choices=("x86_64", "aarch64"))
    args = parser.parse_args()
    apply(args.source, args.destination, args.linux_arch)
