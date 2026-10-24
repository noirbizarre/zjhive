"""Run a disposable real Zellij host and record finite versus streamed HTTP delivery.

No user configuration is changed. Only the uniquely named test session is killed.
Run after compiling transport-probe for wasm32-wasip1.
"""

import http.server
import json
import os
from pathlib import Path
import pty
import re
import select
import subprocess
import tempfile
import threading
import time


def main():
    started = time.monotonic()
    records = []

    class Server(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            records.append({"at": round(time.monotonic() - started, 2), "get": self.path})
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream" if self.path == "/sse" else "text/plain")
            self.end_headers()
            if self.path == "/sse":
                for i in range(3):
                    self.wfile.write(f"data: {i}\n\n".encode())
                    self.wfile.flush()
                    records.append({"at": round(time.monotonic() - started, 2), "emitted": i})
                    time.sleep(2)
                records.append({"at": round(time.monotonic() - started, 2), "eof": True})
            else:
                self.wfile.write(b"finite-ok")

        def do_POST(self):
            body = self.rfile.read(int(self.headers["Content-Length"])).decode()
            records.append({"at": round(time.monotonic() - started, 2), "report": body})
            self.send_response(200)
            self.end_headers()

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 18769), Server)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    root = Path(__file__).resolve().parents[1]
    plugin = root / "target/wasm32-wasip1/release/transport-probe.wasm"
    session = f"zjhive-probe-{os.getpid()}"
    master, slave = pty.openpty()
    # A sized terminal avoids a host startup failure unrelated to the transport.
    import fcntl
    import struct
    import termios
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
    with tempfile.TemporaryDirectory(prefix="zjhive-probe-") as temp:
        config = Path(temp) / "config.kdl"
        config.write_text('on_force_close "quit"\nshow_release_notes false\nshow_startup_tips false\n')
        layout = f'layout {{\n pane {{\n plugin location="file:{plugin}" {{\n url "http://127.0.0.1:18769"\n }}\n }}\n}}\n'
        layout_file = Path(temp) / "layout.kdl"
        layout_file.write_text(layout)
        process = subprocess.Popen(
            ["zellij", "--config", str(config), "--session", session, "--new-session-with-layout", str(layout_file)],
            stdin=slave, stdout=slave, stderr=slave,
            env={**os.environ, "TERM": "xterm-256color", "ZELLIJ_CONFIG_DIR": temp},
            start_new_session=True,
        )
        os.close(slave)
        terminal = bytearray()
        try:
            deadline = time.monotonic() + 25
            while time.monotonic() < deadline and process.poll() is None:
                readable, _, _ = select.select([master], [], [], 0.5)
                if readable:
                    try:
                        terminal.extend(os.read(master, 65536))
                    except OSError:
                        break
                # The only permission dialog is our probe's WebAccess request.
                if b"permission" in terminal.lower():
                    os.write(master, b"y")
                if any("report" in r and "sse: status=200" in r["report"] for r in records):
                    break
        finally:
            subprocess.run(["zellij", "kill-session", session], capture_output=True, check=False)
            process.wait(timeout=10)
            subprocess.run(["zellij", "delete-session", session], capture_output=True, check=False)
            os.close(master)
            server.shutdown()
        print(json.dumps({"host": subprocess.check_output(["zellij", "--version"], text=True).strip(),
                          "records": records, "terminal": re.sub(r"\s+", " ", re.sub(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x1b]*\x1b\\)", "", terminal.decode(errors="replace")))}, indent=2))
        assert any("report" in r and "finite-ok" in r["report"] for r in records), "finite HTTP not demonstrated"
        assert any("report" in r and "sse: status=200" in r["report"] for r in records), "SSE completion not demonstrated"
        eof = next(r["at"] for r in records if r.get("eof"))
        sse_results = [r for r in records if "sse: status=200" in r.get("report", "")]
        assert len(sse_results) == 1 and sse_results[0]["at"] >= eof, "host streaming behavior changed; investigate direct ACP/SSE"
        assert "Unsupported" in sse_results[0]["report"], "direct socket behavior changed; investigate WebSocket"


if __name__ == "__main__":
    main()
