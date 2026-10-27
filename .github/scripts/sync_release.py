"""Keep release versions in README examples and the local FFI dependency aligned."""

import argparse
from pathlib import Path
import re
import tomllib


def synchronize(root: Path, *, check: bool = False) -> list[Path]:
    manifest_path = root / "Cargo.toml"
    manifest_text = manifest_path.read_text()
    version = tomllib.loads(manifest_text)["package"]["version"]
    ffi_version = tomllib.loads(
        (root / "libsystemd-sys/Cargo.toml").read_text()
    )["package"]["version"]

    # A broad requirement such as ^0.9 can select a published sys crate that
    # lacks the FFI declarations used by the new systemd release.
    section = re.search(
        r"(?ms)^\[dependencies\.libsystemd-sys\]\n(.*?)(?=^\[|\Z)",
        manifest_text,
    )
    if section is None:
        raise ValueError("Missing libsystemd-sys dependency table")
    dependency, count = re.subn(
        r'(?m)^(version\s*=\s*")[^"]+(".*)$',
        lambda match: match[1] + ffi_version + match[2],
        section[1],
    )
    if count != 1:
        raise ValueError("Expected one libsystemd-sys version requirement")
    expected = {
        manifest_path: manifest_text[: section.start(1)]
        + dependency
        + manifest_text[section.end(1) :]
    }

    # Each README follows its own crate so a high-level version change does not
    # also change the sys crate's packaged documentation and trigger a release.
    for relative, package, package_version in (
        ("README.md", "systemd", version),
        ("libsystemd-sys/README.md", "libsystemd-sys", ffi_version),
    ):
        path = root / relative
        text, count = re.subn(
            r"(?m)^(" + re.escape(package) + r'\s*=\s*(?:\{\s*version\s*=\s*)?")[^"]+(".*)$',
            lambda match: match[1] + package_version + match[2],
            path.read_text(),
        )
        if count == 0:
            raise ValueError(f"No {package} dependency example in {relative}")
        expected[path] = text

    changed = [path for path, text in expected.items() if path.read_text() != text]
    if not check:
        for path in changed:
            path.write_text(expected[path])
    return changed


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    changed = synchronize(root, check=args.check)
    for path in changed:
        print(path.relative_to(root))
    if args.check and changed:
        parser.exit(1, "Release metadata is out of sync\n")


if __name__ == "__main__":
    main()
