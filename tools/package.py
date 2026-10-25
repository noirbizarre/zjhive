"""Package the already-built, platform-independent WASM plugin and its checksum."""

import hashlib
from pathlib import Path
import shutil
import tomllib


def package(root):
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    wasm = root / "target/wasm32-wasip1/release/zjhive.wasm"
    if not wasm.is_file():
        raise SystemExit("Build the release WASM plugin with mise run plugin:build first")
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    plugin = dist / f"zjhive-{version}.wasm"
    shutil.copyfile(wasm, plugin)
    checksum = dist / f"zjhive-{version}.sha256"
    checksum.write_text(f"{hashlib.sha256(plugin.read_bytes()).hexdigest()}  {plugin.name}\n")
    return plugin, checksum


if __name__ == "__main__":
    for path in package(Path(__file__).resolve().parents[1]):
        print(path)
