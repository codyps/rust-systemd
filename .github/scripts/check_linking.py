"""Exercise explicit native library selection and expected build failures."""

import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lib-dir", type=Path, required=True)
    args = parser.parse_args()
    if not args.lib_dir.is_dir():
        parser.error("--lib-dir must contain the installed libsystemd")

    with tempfile.TemporaryDirectory(prefix="rust-systemd-linking-") as directory:
        cases = [
            ("manual default library", {"SYSTEMD_LIB_DIR": str(args.lib_dir)}, None),
            ("manual dylib", {"SYSTEMD_LIB_DIR": str(args.lib_dir), "SYSTEMD_LIBS": "dylib=systemd"}, None),
            ("manual library list", {"SYSTEMD_LIB_DIR": str(args.lib_dir), "SYSTEMD_LIBS": "dylib=systemd:dylib=c"}, None),
            ("missing pkg-config package", {"SYSTEMD_PKG_NAME": "rust_systemd_missing"}, "pkg_config could not find"),
            ("missing library directory", {"SYSTEMD_LIB_DIR": str(Path(directory) / "missing")}, "which does not exist"),
            ("missing explicit library", {"SYSTEMD_LIB_DIR": str(args.lib_dir), "SYSTEMD_LIBS": "dylib=rust_systemd_missing"}, "rust_systemd_missing"),
        ]
        for name, overrides, expected_failure in cases:
            environment = os.environ.copy()
            for variable in ("SYSTEMD_LIB_DIR", "SYSTEMD_LIBS", "SYSTEMD_PKG_NAME"):
                environment.pop(variable, None)
            # Every manual mode must work even when pkg-config cannot be used.
            environment["PKG_CONFIG"] = "/bin/false"
            environment.update(overrides)
            result = subprocess.run(
                ["cargo", "test", "--locked", "-p", "libsystemd-sys", "--test", "journal-send", "--features", "journal"],
                env=environment,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                check=False,
            )
            if expected_failure is None:
                passed = result.returncode == 0
            else:
                passed = result.returncode != 0 and expected_failure in result.stdout
            if not passed:
                raise RuntimeError(f"{name}: unexpected result {result.returncode}\n{result.stdout}")
            print(f"{name}: passed", flush=True)


if __name__ == "__main__":
    main()
