"""Enforce the single-artifact and read-only host integration boundaries."""

import hashlib
from pathlib import Path
import tempfile
import unittest

from package import package


class DashboardContract(unittest.TestCase):
    def test_packaging_emits_only_wasm_and_its_matching_checksum(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "Cargo.toml").write_text('[workspace.package]\nversion = "1.2.3"\n')
            target = root / "target/wasm32-wasip1/release"
            target.mkdir(parents=True)
            wasm = b"\0asm\x01\0\0\0"
            (target / "zjhive.wasm").write_bytes(wasm)
            plugin, checksum = package(root)
            self.assertEqual(plugin.read_bytes(), wasm)
            self.assertEqual(checksum.read_text(), f"{hashlib.sha256(wasm).hexdigest()}  {plugin.name}\n")
            self.assertEqual(sorted(p.name for p in (root / "dist").iterdir()),
                             ["zjhive-1.2.3.sha256", "zjhive-1.2.3.wasm"])

    def test_host_integration_cannot_launch_a_native_network_pump_or_mutate_hive(self):
        source = Path(__file__).resolve().parents[1] / "crates/zjhive/src"
        main = (source / "main.rs").read_text()
        self.assertIn("request_permission(&[PermissionType::WebAccess])", main)
        self.assertEqual(main.count("PermissionType::"), 1)
        self.assertIn("HttpVerb::Get", main)
        self.assertEqual(main.count("HttpVerb::"), 1)
        for path in source.glob("*.rs"):
            text = path.read_text()
            for forbidden in ["run_command(", "open_command_pane(", "std::process", "TcpStream", "RunCommands"]:
                self.assertNotIn(forbidden, text, str(path))


if __name__ == "__main__":
    unittest.main()
