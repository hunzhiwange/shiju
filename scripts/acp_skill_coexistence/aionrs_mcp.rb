# Read-only stdio fixture. The production broker allows writes only in its scratch HOME.
require "json"

attachment = ARGV.fetch(0)
journal = File.join(ENV.fetch("HOME"), "coexistence.audit")
ack = File.join(ENV.fetch("HOME"), "coexistence.ack")
STDOUT.sync = true

STDIN.each_line do |line|
  request = JSON.parse(line)
  result = case request.fetch("method")
  when "initialize"
    {protocolVersion: request.fetch("params").fetch("protocolVersion"),
     capabilities: {tools: {}}, serverInfo: {name: "coexistence", version: "1"}}
  when "notifications/initialized"
    next
  when "tools/list"
    {tools: [{name: "read_probe", description: "Read a private fixture file",
              inputSchema: {type: "object", properties: {}, additionalProperties: false}}]}
  when "tools/call"
    params = request.fetch("params")
    abort unless params.fetch("name") == "read_probe" && params.fetch("arguments").empty?
    File.open(journal, "a") { |file| file.puts("stdio") }
    count = File.readlines(journal).length
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 5
    until File.file?(ack) && File.read(ack).to_i == count
      abort if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
      sleep(0.01)
    end
    {content: [{type: "text", text: File.read(attachment)}], isError: false}
  else
    abort
  end
  puts JSON.generate({jsonrpc: "2.0", id: request.fetch("id"), result: result})
end
