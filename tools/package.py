"""Package already-built matching plugin/companion artifacts; never builds Hive."""

import argparse
import hashlib
from pathlib import Path
import shutil
import tarfile
import tomllib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True, help="native Rust target triple")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    wasm = root / "target/wasm32-wasip1/release/zjhive.wasm"
    # Require the named target directory so an archive cannot be mislabeled by a host fallback.
    native = root / "target" / args.target / "release/zjhive-bridge"
    if not wasm.is_file() or not native.is_file():
        parser.error("build the release plugin and native companion before packaging")
    plugin = dist / f"zjhive-{version}.wasm"
    shutil.copyfile(wasm, plugin)
    archive = dist / f"zjhive-{version}-{args.target}.tar.gz"
    with tarfile.open(archive, "w:gz") as output:
        for source, name in [(native, "zjhive-bridge"), (plugin, "zjhive.wasm"),
                             (root / "layouts/zjhive.kdl", "zjhive.kdl"),
                             (root / "README.md", "README.md"), (root / "LICENSE", "LICENSE")]:
            output.add(source, arcname=name)
    checksums = dist / f"zjhive-{version}-{args.target}.sha256"
    checksums.write_text("".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n"
                                 for path in [plugin, archive]))
    print(f"Packaged {plugin.name}, {archive.name}, {checksums.name}")


if __name__ == "__main__":
    main()
