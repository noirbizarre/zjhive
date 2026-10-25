"""Exercise the actual WASM dashboard in Zellij using finite HTTP fixtures only."""

import fcntl
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import threading
import time


def main():
    root = Path(__file__).resolve().parents[1]
    plugin = root / "target/wasm32-wasip1/release/zjhive.wasm"
    if not plugin.is_file():
        raise SystemExit("Run mise run plugin:build first")
    state = {"stage": "initial", "requests": []}

    class Handler(BaseHTTPRequestHandler):
        def parse_request(self):
            parsed = super().parse_request()
            # Dispatch may reject a method without calling any of our handlers.
            if parsed:
                state["requests"].append((self.command, self.path))
            return parsed

        def do_GET(self):
            # Only the CLI-loaded instance receives this prefix, so its output is unique.
            cli = self.path.startswith("/cli/")
            path = self.path.removeprefix("/cli") if cli else self.path
            stage = "cli" if cli else state["stage"]
            if path == "/v1/status":
                data = {"name": "hive", "version": "fixture-1", "sessions": 2,
                        "defaultAgent": "fixture", "activeByAgent": {"fixture": 2}}
            elif path == "/v1/sessions":
                if stage == "failure":
                    self.send_response(503)
                    self.end_headers()
                    return
                sessions = [{"sessionId": f"session-{id_}", "agent": "fixture", "cwd": "/fixture",
                             "title": f"{stage}-title-{id_}", "lifecycle": "active", "activity": "idle",
                             "recovery": "reattached", "observers": 0, "generation": 1}
                            for id_ in ("a", "b")]
                if stage == "reordered":
                    sessions.reverse()
                if stage == "empty":
                    sessions = []
                data = {"revision": 1, "sessions": sessions}
            else:
                self.send_error(404)
                return
            body = json.dumps(data).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_port}"
    session = f"zjhive-dashboard-{os.getpid()}"
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 180, 0, 0))
    observations = []
    permission_granted = False

    def wait_for(text, timeout=25):
        nonlocal permission_granted
        stage = bytearray()
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            readable, _, _ = select.select([master], [], [], 0.2)
            if readable:
                stage.extend(os.read(master, 65536))
                plain = re.sub(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x1b]*\x1b\\)", "", stage.decode(errors="replace"))
                if "permission" in plain.lower() and not permission_granted:
                    os.write(master, b"y")
                    permission_granted = True
                if text in plain:
                    observations.append(text)
                    return
        raise AssertionError(f"host never displayed {text!r}: {stage.decode(errors='replace')!r}")

    try:
        with tempfile.TemporaryDirectory(prefix="zjhive-dashboard-") as temp:
            config = Path(temp) / "config.kdl"
            # Locked mode forwards navigation keys to the plugin rather than Zellij modes.
            config.write_text('default_mode "locked"\nshow_release_notes false\nshow_startup_tips false\n')
            layout = Path(temp) / "layout.kdl"
            layout.write_text(f'layout {{\n pane {{\n plugin location="file:{plugin}" {{\n url "{url}"\n }}\n }}\n}}\n')
            host = subprocess.Popen(["zellij", "--config", str(config), "--session", session,
                                     "--new-session-with-layout", str(layout)],
                                    stdin=slave, stdout=slave, stderr=slave,
                                    env={**os.environ, "TERM": "xterm-256color", "ZELLIJ_CONFIG_DIR": temp},
                                    start_new_session=True)
            os.close(slave)
            slave = None
            try:
                wait_for("initial-title-a")
                assert any(path == "/v1/status" for _, path in state["requests"])
                os.write(master, b"\r")
                wait_for("Session: session-a")
                state["stage"] = "reordered"
                os.write(master, b"r")
                wait_for("reordered-title-a")
                state["stage"] = "failure"
                os.write(master, b"r")
                wait_for("STALE")
                state["stage"] = "recovered"
                os.write(master, b"r")
                wait_for("recovered-title-a")
                state["stage"] = "empty"
                os.write(master, b"r")
                wait_for("No sessions listed")
                # Exercise the documented CLI entry point as well as startup layouts.
                subprocess.run(["zellij", "--session", session, "plugin", "--configuration",
                                f"url={url}/cli", "--", f"file:{plugin}"], capture_output=True, check=True)
                wait_for("cli-title-a")
                assert ("GET", "/cli/v1/sessions") in state["requests"]
                assert all(method == "GET" and path in {
                               "/v1/sessions", "/v1/status", "/cli/v1/sessions", "/cli/v1/status"}
                           for method, path in state["requests"]), state["requests"]
                observations.append("only finite operational GETs; no ACP or mutation")
            finally:
                subprocess.run(["zellij", "kill-session", session], capture_output=True, check=False)
                host.wait(timeout=10)
                subprocess.run(["zellij", "delete-session", session], capture_output=True, check=False)
    finally:
        server.shutdown()
        server.server_close()
        if slave is not None:
            os.close(slave)
        os.close(master)
        print(json.dumps({"host": "zellij", "observations": observations,
                          "requests": state["requests"]}, indent=2))


if __name__ == "__main__":
    main()
