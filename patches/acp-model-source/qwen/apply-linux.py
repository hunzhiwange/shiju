#!/usr/bin/env python3
"""Require Qwen's upstream private tool guard on Linux managed launches."""

import hashlib
from pathlib import Path
import sys

AGENT = "chunks/acpAgent-C3CM2WHA.js"
V2_SHA256 = "e81ff979d5633c2c7d4e5afb44e602f706aacac8e5a81e1ae58d0bd24e98e65d"
MANAGED = 'process.platform==="linux"&&process.env.SHIJU_QWEN_MANAGED==="1"'


def insert_after(source, context, addition):
    if source.count(context) != 1:
        raise SystemExit("Qwen Linux patch context mismatch")
    return source.replace(context, context + addition)


def patch_agent(data):
    if hashlib.sha256(data).hexdigest() != V2_SHA256:
        raise SystemExit("Qwen Linux overlay requires the reviewed v2 agent chunk")
    source = data.decode()

    # Attaching the provider is essential. The upstream guard's fallback checks
    # shell tools only. Validate this before the bootstrap config initializes.
    source = insert_after(
        source,
        "const externalToolGuardProviderAttached=options?.externalToolGuardProviderAttached===true;",
        f"""if({MANAGED}){{
          if(!privateParentCapability||!externalToolGuardRequired||!externalToolGuardProviderAttached)
            throw new Error("Managed Qwen requires an attached host tool guard");
        }}""",
    )

    # The stock agent allows an omitted context. In the managed route each
    # prompt must be current and tied to the authenticated ACP session.
    source = insert_after(
        source,
        'const invocationContext=this.privateParentState==="trusted"&&suppliedContext!==void 0?'
        "parseInvocationContext(suppliedContext):void 0;",
        f"""if(({MANAGED})&&invocationContext?.sessionId!==sessionId)
          throw RequestError.invalidParams(void 0,"Managed Qwen requires a bound prompt context");""",
    )
    return source.encode()


if __name__ == "__main__":
    agent = Path(sys.argv[1]) / AGENT
    agent.write_bytes(patch_agent(agent.read_bytes()))
