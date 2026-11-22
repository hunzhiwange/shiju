// Accept only MCP definitions minted by the request-owned host projection.
import { Agent, request as send } from 'node:http';

const direct = new Agent({ keepAlive: false });
const namePattern = /^[a-zA-Z0-9]+(?:_[a-zA-Z0-9]+)*$/;
const maxBody = 4 * 1024 * 1024;

export function selectProjectedServers(servers, endpoint, modelToken, launchReference) {
  const relay = new URL(endpoint);
  if (!launchReference || !modelToken || relay.protocol !== 'http:' ||
      relay.hostname !== '127.0.0.1' || !relay.port || relay.pathname !== '/' ||
      relay.username || relay.password || relay.search || relay.hash ||
      !Array.isArray(servers) || !servers.length || servers.length > 16) {
    throw new Error('Nova requires an owned loopback MCP projection');
  }
  const names = new Set();
  return servers.map(({ type, name, url, headers }) => {
    const header = headers?.[0];
    if (type !== 'http' || typeof name !== 'string' || name.length > 64 ||
        !namePattern.test(name) || names.has(name.toLowerCase()) ||
        url !== `${relay.origin}/shiju-vtcode-mcp/${name}` ||
        !Array.isArray(headers) || headers.length !== 1 || header?.name !== 'Authorization' ||
        typeof header.value !== 'string' || !/^Bearer [0-9a-f]{64}$/.test(header.value) ||
        header.value === `Bearer ${modelToken}`) {
      throw new Error('Invalid Nova host MCP selection');
    }
    names.add(name.toLowerCase());
    return { name, url, authorization: header.value };
  });
}

export async function projectedOperation(request, signal) {
  signal.throwIfAborted();
  const { server, operation, name, arguments: args, callId } = request;
  const list = operation === 'list';
  if (!list && (operation !== 'call' || typeof name !== 'string' || !namePattern.test(name) ||
      typeof callId !== 'string' || !callId || !args || typeof args !== 'object' || Array.isArray(args))) {
    throw new Error('Invalid Nova MCP invocation');
  }
  const params = list ? {} : {
    name, arguments: args, _meta: { 'shiju.nova.toolCallId': callId },
  };
  const body = JSON.stringify({
    jsonrpc: '2.0', id: 1, method: list ? 'tools/list' : 'tools/call', params,
  });
  if (Buffer.byteLength(body) > maxBody) throw new Error('Nova MCP request too large');
  // A dedicated direct HTTP agent keeps inherited proxy settings out of this connection.
  const response = await new Promise((resolve, reject) => {
    const call = send(server.url, {
      agent: direct, method: 'POST', signal: AbortSignal.any([signal, AbortSignal.timeout(20000)]),
      headers: {
        Authorization: server.authorization, 'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(body),
      },
    }, resolve);
    call.once('error', reject);
    call.end(body);
  });
  if (response.statusCode !== 200) {
    response.destroy();
    throw new Error('Nova MCP projection rejected the request');
  }
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.byteLength;
    if (bytes > maxBody) throw new Error('Nova MCP response too large');
    chunks.push(chunk);
  }
  signal.throwIfAborted();
  const reply = JSON.parse(Buffer.concat(chunks, bytes).toString('utf8'));
  if (reply?.jsonrpc !== '2.0' || reply.id !== 1 || reply.error ||
      !reply.result || typeof reply.result !== 'object' || Array.isArray(reply.result) ||
      (list && !Array.isArray(reply.result.tools))) {
    throw new Error('Invalid Nova MCP projection response');
  }
  return list ? reply.result.tools : reply.result;
}
