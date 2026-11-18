"""Run a single operation using the reviewed Cortex stdio sandbox."""
import asyncio
import json
from pathlib import Path
import runpy
import signal
import sys


bridge = runpy.run_path(str(Path(__file__).with_name('shiju-stdio.py')))


async def execute(request):
    peer = bridge['Stdio'](request['server'], Path(request['workspace']))
    async def operation():
        if request['operation'] != 'list':
            return await peer.call(request['name'], request['arguments'])
        tools = []
        async for tool in peer.list_tools():
            tools.append(tool)
            if len(tools) > 256:
                raise ValueError('MCP catalog exceeds limit')
        return tools

    try:
        return await asyncio.wait_for(operation(), bridge['TIMEOUT'])
    finally:
        await peer.close()


async def main():
    loop = asyncio.get_running_loop()
    reader = asyncio.StreamReader(limit=bridge['LIMIT'])
    await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin.buffer)
    request = json.loads(await reader.readline())
    operation = asyncio.create_task(execute(request))
    disconnect = asyncio.create_task(reader.read(1))
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, operation.cancel)
    try:
        done, _ = await asyncio.wait((operation, disconnect), return_when=asyncio.FIRST_COMPLETED)
        if operation in done:
            print(json.dumps(operation.result()))
    finally:
        operation.cancel()
        disconnect.cancel()
        await asyncio.gather(operation, disconnect, return_exceptions=True)


if __name__ == '__main__':
    try:
        asyncio.run(main())
    except (Exception, asyncio.CancelledError):
        sys.exit(1)
