"""Exercise the packaged plugin/companion in Zellij against an existing Hive session.

The daemon is supplied through --url. A dedicated session and bridge process are
created and cleaned up; the Hive session itself is never closed.
"""

import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import socket
import struct
import subprocess
import tempfile
import termios
import time
import urllib.request


def main():
    args = argparse.ArgumentParser()
    args.add_argument("--url", required=True)
    options = args.parse_args()
    root = Path(__file__).resolve().parents[1]
    plugin = root / "target/wasm32-wasip1/release/zjhive.wasm"
    binary = root / "target/debug/zjhive-bridge"
    with socket.socket() as reserve:
        reserve.bind(("127.0.0.1", 0))
        listen = f"127.0.0.1:{reserve.getsockname()[1]}"
    companion_command = [str(binary), "--listen", listen, "--url", options.url]
    companion = subprocess.Popen(companion_command)
    session = f"zjhive-live-{os.getpid()}"
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 160, 0, 0))
    observations = []

    def wait_for(text, timeout=25):
        stage = bytearray()
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            readable, _, _ = select.select([master], [], [], 0.2)
            if readable:
                stage.extend(os.read(master, 65536))
                plain = re.sub(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x1b]*\x1b\\)", "", stage.decode(errors="replace"))
                if "permission" in plain.lower():
                    os.write(master, b"y")
                if text in plain:
                    observations.append(text)
                    return
        plain = re.sub(r"\s+", " ", re.sub(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x1b]*\x1b\\)", "", stage.decode(errors="replace")))
        raise AssertionError(f"host never displayed {text!r}: {plain}")

    try:
        with tempfile.TemporaryDirectory(prefix="zjhive-live-") as temp:
            config = Path(temp) / "config.kdl"
            # Locked mode forwards Ctrl-T to the plugin instead of entering Zellij tab mode.
            config.write_text('default_mode "locked"\nshow_release_notes false\nshow_startup_tips false\n')
            layout = Path(temp) / "layout.kdl"
            layout.write_text(f'layout {{\n pane {{\n plugin location="file:{plugin}" {{\n url "{options.url}"\n bridge "http://{listen}"\n }}\n }}\n}}\n')
            host = subprocess.Popen(["zellij", "--config", str(config), "--session", session,
                                     "--new-session-with-layout", str(layout)],
                                    stdin=slave, stdout=slave, stderr=slave,
                                    env={**os.environ, "TERM": "xterm-256color", "ZELLIJ_CONFIG_DIR": temp},
                                    start_new_session=True)
            os.close(slave)
            try:
                wait_for("Select a Hive session")
                os.write(master, b"\r")
                wait_for("Observer")
                os.write(master, b"\x14")
                wait_for("Pilot")
                os.write(master, b"live smoke\r")
                wait_for("Prompt accepted")
                # Losing the local adapter must not kill the authoritative Hive session.
                companion.terminate()
                companion.wait(timeout=10)
                wait_for("STALE")
                companion = subprocess.Popen(companion_command)
                wait_for("Observer", timeout=45)
                os.write(master, b"\x1b")
                wait_for("Detached; the Hive session remains running")
                with urllib.request.urlopen(f"{options.url}/v1/sessions") as response:
                    catalog = json.load(response)
                assert catalog["sessions"], "detaching destroyed the session"
                observations.append("Hive session survives companion restart and detach")
                # Exercise the documented CLI load syntax as well as the startup-layout path.
                subprocess.run(["zellij", "--session", session, "plugin", "--configuration",
                                f"url={options.url},bridge=http://{listen}", "--", f"file:{plugin}"],
                               capture_output=True, check=True)
                wait_for("Select a Hive session")
                observations.append("documented zellij plugin CLI load succeeds")
            finally:
                subprocess.run(["zellij", "kill-session", session], capture_output=True, check=False)
                host.wait(timeout=10)
                subprocess.run(["zellij", "delete-session", session], capture_output=True, check=False)
    finally:
        companion.terminate()
        companion.wait(timeout=10)
        os.close(master)
        print(json.dumps({"host": "zellij 0.45.1", "observations": observations}, indent=2))


if __name__ == "__main__":
    main()
