"""Exercise Serde as a downstream dependency without workspace dev-feature unification."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="rust-systemd-serde-") as temporary:
        directory = Path(temporary)
        (directory / "src").mkdir()
        manifest = directory / "Cargo.toml"
        manifest.write_text(
            '[package]\nname = "serde-feature-consumer"\nversion = "0.0.0"\nedition = "2018"\n'
            '[dependencies]\n'
            f'systemd = {{ path = {json.dumps(str(ROOT))}, default-features = false, features = ["serde"] }}\n'
            'serde = { version = "1", default-features = false }\n'
            '[features]\n'
            'systemd-defaults = ["systemd/default"]\n'
        )
        # Keep dependency versions aligned without selecting the workspace's dev-dependencies.
        shutil.copyfile(ROOT / "Cargo.lock", directory / "Cargo.lock")
        (directory / "src/main.rs").write_text(r'''
use serde::{de::value::{Error, StrDeserializer}, Deserialize, Serialize};
use systemd::id128::Id128;

fn assert_traits<T: Serialize + for<'de> Deserialize<'de>>() {}

fn main() {
    assert_traits::<Id128>();
    let text = "0123456789abcdef0123456789abcdef";
    let id = Id128::deserialize(StrDeserializer::<Error>::new(text)).unwrap();
    assert_eq!(id.to_string(), text);
    assert!(Id128::deserialize(StrDeserializer::<Error>::new("invalid")).is_err());
    assert!(Id128::deserialize(StrDeserializer::<Error>::new("interior\0nul")).is_err());
}
''')
        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(directory / "target")
        for name, features in [
            ("standalone serde without default systemd features", []),
            ("standalone serde with default systemd features", ["systemd-defaults"]),
        ]:
            command = ["cargo", "run", "--offline", "--manifest-path", str(manifest)]
            if features:
                command += ["--features", ",".join(features)]
            result = subprocess.run(
                command, env=environment, stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT, text=True, check=False,
            )
            if result.returncode:
                raise RuntimeError(f"{name}: consumer failed\n{result.stdout}")
            print(f"{name}: passed", flush=True)


if __name__ == "__main__":
    main()
