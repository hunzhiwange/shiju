#!/usr/bin/env python3
"""Run a separately installed acpx CLI against shiju's real ACP stdio server."""
import argparse
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import sqlite3
import subprocess
import tempfile
import threading
import time

from verify_acp_integration import Process, Provider, ROOT, prepare, require, text_updates


def terminal_text(data):
    return re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", data.decode(errors="replace")).replace("\r", "")


class ExternalClient:
    def __init__(self, acpx, shiju, output, env, cwd):
        self.acpx, self.output, self.env, self.cwd = acpx, output, env, cwd
        self.base = [str(acpx), "--agent", f"{shiju} acp", "--cwd", str(cwd),
                     "--model", "fixture/alpha", "--timeout", "20", "--ttl", "15"]
        self.calls = []

    def command(self, args):
        return self.base + args

    def run(self, name, args, exits=(0,)):
        result = subprocess.run(self.command(args), cwd=self.cwd, env=self.env,
                                capture_output=True, text=True, timeout=30)
        (self.output / f"{name}.stdout").write_text(result.stdout)
        (self.output / f"{name}.stderr").write_text(result.stderr)
        self.calls.append({"name": name, "args": args, "exit": result.returncode})
        (self.output / "client-calls.json").write_text(json.dumps(self.calls, indent=2))
        require(result.returncode in exits, f"{name}: exit={result.returncode}; {result.stderr}")
        return result.stdout

    def interactive(self, name, marker, answer):
        master, slave = pty.openpty()
        child = subprocess.Popen(self.command(["--format", "text", "exec", marker]),
                                 cwd=self.cwd, env=self.env, stdin=slave, stdout=slave,
                                 stderr=slave, start_new_session=True)
        os.close(slave)
        transcript = b""
        answered = False
        deadline = time.monotonic() + 30
        try:
            while time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    try:
                        data = os.read(master, 65536)
                    except OSError:
                        break
                    if not data:
                        break
                    transcript += data
                    if not answered and b"(y/N)" in transcript:
                        require(not (self.cwd / f"{marker}.txt").exists(), "no write before confirmation")
                        os.write(master, answer.encode() + b"\n")
                        answered = True
                elif child.poll() is not None:
                    break
            require(answered, f"{name}: real terminal permission question missing")
            expected = 0 if answer == "y" else 5
            require(child.wait(timeout=5) == expected, f"{name}: terminal decision exit")
            text = terminal_text(transcript)
            require("[edit]" in text, f"{name}: edit permission kind")
            require(f"[tool] file_write ({'completed' if answer == 'y' else 'failed'})" in text,
                    f"{name}: visible terminal tool result")
            require((self.cwd / f"{marker}.txt").exists() == (answer == "y"), f"{name}: actual side effect")
        finally:
            (self.output / f"{name}.terminal.txt").write_bytes(transcript)
            (self.output / f"{name}.plain.txt").write_text(terminal_text(transcript))
            if child.poll() is None:
                child.kill()
                child.wait()
            os.close(master)

    def owners(self):
        return [(path, json.loads(path.read_text())) for path in
                (Path(self.env["HOME"]) / ".acpx/queues").glob("*.lock")]

    def disconnect_permission(self):
        master, slave = pty.openpty()
        child = subprocess.Popen(self.command(["--format", "json", "exec", "TOOL_DISCONNECT"]),
                                 cwd=self.cwd, env=self.env, stdin=slave, stdout=slave,
                                 stderr=slave, start_new_session=True)
        os.close(slave)
        transcript = b""
        server_pid = None
        try:
            deadline = time.monotonic() + 20
            while b"(y/N)" not in transcript and time.monotonic() < deadline:
                if select.select([master], [], [], .1)[0]:
                    transcript += os.read(master, 65536)
            require(b"(y/N)" in transcript, "pending real confirmation before disconnect")
            frames = [json.loads(line) for line in terminal_text(transcript).splitlines() if line.startswith('{"')]
            sid = next(frame["result"]["sessionId"] for frame in frames
                       if isinstance(frame.get("result"), dict) and "sessionId" in frame["result"])
            processes = subprocess.check_output(["ps", "-axo", "pid=,ppid=,command="], text=True)
            for line in processes.splitlines():
                fields = line.strip().split(None, 2)
                if len(fields) == 3 and fields[1] == str(child.pid) and fields[2].startswith(self.base[2]):
                    server_pid = int(fields[0])
            require(server_pid is not None, "isolated real client agent child")
            child.kill()
            child.wait(timeout=5)
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                state = subprocess.run(["ps", "-p", str(server_pid), "-o", "stat="],
                                       capture_output=True, text=True).stdout.strip()
                if not state or state.startswith("Z"):
                    break
                time.sleep(.05)
            else:
                raise AssertionError("agent must exit and persist after client disconnect")
        finally:
            (self.output / "permission-disconnect.terminal.txt").write_bytes(transcript)
            if child.poll() is None:
                child.kill()
                child.wait()
            os.close(master)
        require(not (self.cwd / "TOOL_DISCONNECT.txt").exists(), "disconnection cannot approve pending write")
        loaded = self.run("load-disconnected", ["--format", "json", "sessions", "new", "--name",
                          "permission-recovery", "--resume-session", sid])
        require(json.loads(loaded)["acpxSessionId"] == sid, "native session identity is restored")
        resumed = self.run("resume-disconnected", ["--format", "json", "prompt", "-s",
                           "permission-recovery", "Continue after permission disconnect"])
        frames = [json.loads(line) for line in resumed.splitlines()]
        # acpx controls which replay notifications reach its formatter; verify the
        # actual load request and provider-visible history rather than its UI cache.
        require(any(frame.get("method") == "session/load" and frame["params"]["sessionId"] == sid
                    for frame in frames), "real client reloads the original native session")
        require("ACK_alpha;USERS=2;SLOW=False" in text_updates(frames),
                "real client resumes after pending permission disconnection")
        require(not (self.cwd / "TOOL_DISCONNECT.txt").exists(), "reconnection cannot revive pending write")
        self.run("close-disconnected", ["--format", "json", "sessions", "close", "permission-recovery"])

    def owner(self, sid):
        owners = [record for _, record in self.owners() if record.get("sessionId") == sid]
        require(len(owners) == 1, "one isolated external queue owner")
        owner = owners[0]
        command = subprocess.run(["ps", "-p", str(owner["pid"]), "-o", "command="],
                                 capture_output=True, text=True, check=True).stdout
        require(str(self.acpx.parent.parent) in command, "fault injection targets installed test client")
        return owner

    def close(self):
        for _, record in self.owners():
            pid = record.get("pid")
            if not isinstance(pid, int):
                continue
            command = subprocess.run(["ps", "-p", str(pid), "-o", "command="],
                                     capture_output=True, text=True).stdout
            if str(self.acpx.parent.parent) in command:
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass


def scenarios(client):
    stdout = client.run("suppressed-read", ["--suppress-reads", "--format", "text", "exec", "TOOL_READ"])
    tool_blocks = stdout.split("[tool] file_read (completed)", 1)[-1].split("[thinking]", 1)[0]
    require("[tool] file_read (completed)" in stdout, "real read tool completes")
    require("[read output suppressed]" in tool_blocks and "ACP_TOOL_FIXTURE" not in tool_blocks,
            "real client suppresses read output by explicit tool kind")
    (client.cwd / "long-unicode.txt").write_text("a" * 1999 + "中文尾部")
    grep = client.run("unicode-grep", ["--format", "text", "exec", "TOOL_GREP_UNICODE"])
    require("[tool] grep (completed)" in grep and "[done] end_turn" in grep,
            "real ACP server survives grep at a multibyte truncation boundary")
    database = Path(client.env["VIBEWINDOW_CONFIG_DIR"]) / "database/vibe-window.sqlite3"
    with sqlite3.connect(database) as connection:
        config = json.loads(connection.execute("SELECT payload_json FROM system_configs").fetchone()[0])
        config["autonomy"]["always_ask"].append("file_read")
        connection.execute("UPDATE system_configs SET payload_json=?", (json.dumps(config),))
    approved_read = client.run("read-approval", ["--format", "json", "exec", "TOOL_READ"])
    frames = [json.loads(line) for line in approved_read.splitlines()]
    require(any(frame.get("method") == "session/request_permission" and
                frame["params"]["toolCall"].get("kind") == "read" for frame in frames),
            "explicit read permission kind allows default read policy without a terminal")
    client.interactive("permission-allow", "TOOL_ALLOW", "y")
    client.interactive("permission-reject", "TOOL_REJECT", "n")
    stdout = client.run("noninteractive-reject", ["--format", "text", "exec", "TOOL_INVALID"], exits=(5,))
    require("[tool] file_write (failed)" in stdout, "unavailable confirmation fails closed")
    require(not (client.cwd / "TOOL_INVALID.txt").exists(), "no noninteractive write")
    created = json.loads(client.run("create", ["--format", "json", "sessions", "new", "--name", "reconnect"]))
    sid = created["acpxRecordId"]
    first = client.run("first", ["--format", "json", "prompt", "-s", "reconnect", "MARKER_FIRST external"])
    require("ACK_alpha;USERS=1;SLOW=False" in text_updates([json.loads(line) for line in first.splitlines()]),
            "real named session starts")
    # Kill only the owner admitted by this isolated HOME; its agent observes stdin EOF.
    owner = client.owner(sid)
    os.kill(owner["pid"], signal.SIGKILL)
    resumed = client.run("reconnected", ["--format", "json", "prompt", "-s", "reconnect", "Continue after owner crash"])
    require("ACK_alpha;USERS=2;SLOW=False" in text_updates([json.loads(line) for line in resumed.splitlines()]),
            "real client reconnects and restores native history")
    slow = Process(client.command(["--format", "json", "prompt", "-s", "reconnect", "SLOW_CANCEL external"]),
                   client.cwd, client.env, client.output / "active-owner-crash.log")
    try:
        deadline = time.monotonic() + 15
        while True:
            frame = slow.frames.get(timeout=max(.01, deadline - time.monotonic()))
            require("eof" not in frame, "real active stream before queue crash")
            if any("SLOW=True" in text for text in text_updates([frame])):
                break
        owner = client.owner(sid)
        os.kill(owner["pid"], signal.SIGKILL)
        require(slow.child.wait(timeout=5) != 0, "lost external queue reports failure")
    finally:
        slow.close()
    resumed = client.run("reconnected-active", ["--format", "json", "prompt", "-s", "reconnect",
                         "Continue after streaming disconnect"])
    require("ACK_alpha;USERS=4;SLOW=False" in text_updates([json.loads(line) for line in resumed.splitlines()]),
            "real active disconnect retains interrupted user prompt")
    client.run("close", ["--format", "json", "sessions", "close", "reconnect"])
    client.disconnect_permission()
    return {"read_display_suppression": True, "unicode_grep_no_crash": True,
            "read_permission_kind": True, "interactive_allow": True,
            "interactive_reject": True, "noninteractive_reject": True,
            "owner_crash_reconnect": True, "active_disconnect_reconnect": True,
            "permission_disconnect_reconnect": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--acpx", type=Path, required=True, help="Separately installed acpx executable")
    parser.add_argument("--shiju", type=Path, default=ROOT / "target-dir/debug/shiju")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    acpx, shiju = args.acpx.absolute(), args.shiju.resolve(strict=True)
    require(acpx.is_file(), "external client executable exists")
    version = subprocess.check_output([str(acpx), "--version"], text=True).strip()
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix="vw-acp-external-"))
    output.mkdir(parents=True, exist_ok=not args.output)
    (output / "client-version.txt").write_text(version + "\n")
    server = Provider(output)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    env, cwd = prepare(output, server.server_port)
    (cwd / "fixture.txt").write_text("ACP_TOOL_FIXTURE")
    client = ExternalClient(acpx, shiju, output, env, cwd)
    print(f"External client: acpx {version}; evidence: {output}", flush=True)
    try:
        results = scenarios(client)
        (output / "results.json").write_text(json.dumps(results, indent=2))
        print(json.dumps(results), flush=True)
    finally:
        client.close()
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
