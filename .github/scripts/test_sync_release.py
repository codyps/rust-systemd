from pathlib import Path
import tempfile
import unittest

from sync_release import synchronize


class ReleaseMetadataTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "libsystemd-sys").mkdir()
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "systemd"\nversion = "0.11.0"\n\n'
            '[dependencies.libsystemd-sys]\npath = "libsystemd-sys"\n'
            'version = "^0.9"\n\n[dev-dependencies]\nversion-sync = "0.9.2"\n'
        )
        (self.root / "libsystemd-sys/Cargo.toml").write_text(
            '[package]\nname = "libsystemd-sys"\nversion = "0.9.5"\n'
        )
        self.examples = (
            'systemd = "0.10.1"\n'
            'systemd = { version = "0.10.1", default-features = false }\n'
        )
        for path in ("README.md", "libsystemd-sys/README.md"):
            (self.root / path).write_text(self.examples)

    def test_versions_are_independent_and_dependency_minimum_is_updated(self):
        changed = synchronize(self.root)
        self.assertEqual(len(changed), 3)
        manifest = (self.root / "Cargo.toml").read_text()
        self.assertIn('version = "0.9.5"', manifest)
        self.assertIn('version-sync = "0.9.2"', manifest)
        for path in ("README.md", "libsystemd-sys/README.md"):
            self.assertEqual(
                (self.root / path).read_text(),
                self.examples.replace("0.10.1", "0.11.0"),
            )
        self.assertEqual(synchronize(self.root), [])

    def test_check_does_not_write(self):
        self.assertEqual(len(synchronize(self.root, check=True)), 3)
        self.assertEqual((self.root / "README.md").read_text(), self.examples)
        self.assertIn('version = "^0.9"', (self.root / "Cargo.toml").read_text())

    def test_missing_dependency_version_fails_before_writing(self):
        path = self.root / "Cargo.toml"
        path.write_text(path.read_text().replace('version = "^0.9"\n', ""))
        with self.assertRaisesRegex(ValueError, "version requirement"):
            synchronize(self.root)
        self.assertEqual((self.root / "README.md").read_text(), self.examples)


if __name__ == "__main__":
    unittest.main()
