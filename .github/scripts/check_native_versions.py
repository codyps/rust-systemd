"""Check native symbol requirements and API gates with synthetic library exports.

Fixtures model symbol availability, not libsystemd runtime behavior. No fixture
library or link-probe executable is run.
"""

import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[2]
# Native ABI introduction versions, independent of the Rust feature conditions.
VERSIONS = {
    "sd_id128_get_machine_app_specific": 233,
    "sd_bus_get_n_queued_read": 238,
    "sd_bus_get_n_queued_write": 238,
    "sd_bus_slot_get_floating": 239,
    "sd_bus_slot_set_floating": 239,
    "sd_bus_slot_set_destroy_callback": 239,
    "sd_bus_slot_get_destroy_callback": 239,
    "sd_bus_track_set_destroy_callback": 239,
    "sd_bus_track_get_destroy_callback": 239,
    "sd_id128_get_boot_app_specific": 240,
    "sd_bus_set_close_on_exit": 240,
    "sd_bus_get_close_on_exit": 240,
    "sd_bus_set_method_call_timeout": 240,
    "sd_bus_get_method_call_timeout": 240,
    "sd_bus_close_unref": 241,
    "sd_journal_open_namespace": 245,
    "sd_session_get_start_time": 254,
}


def run(command, **kwargs):
    return subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, check=False, **kwargs)


def main():
    compiler = shlex.split(os.environ.get("CC", "cc"))
    archiver = shlex.split(os.environ.get("AR", "ar"))
    symbols = set()
    for path in (ROOT / "libsystemd-sys/src").rglob("*.rs"):
        symbols.update(re.findall(r"(?m)^\s*pub fn (sd_\w+)\(", path.read_text()))
    with tempfile.TemporaryDirectory(prefix="rust-systemd-versions-") as temporary:
        directory = Path(temporary)
        base_env = os.environ.copy()
        for key in list(base_env):
            if key.startswith(("SYSTEMD_", "HOST_SYSTEMD_", "TARGET_SYSTEMD_", "PKG_CONFIG_", "LIBSYSTEMD_", "LIBELOGIND_")):
                base_env.pop(key)
        base_env["CARGO_TARGET_DIR"] = str(directory / "target")
        fixtures = {}
        for version in [232, 233, 237, 239, 240, 241, 245, 254]:
            fixture = directory / str(version)
            fixture.mkdir()
            source = fixture / "exports.c"
            source.write_text("\n".join(
                f"void {name}(void) {{}}" for name in sorted(symbols)
                if VERSIONS.get(name, 209) <= version
            ))
            result = run(compiler + ["-shared", "-fPIC", str(source), "-o", str(fixture / "libsystemd.so")])
            if result.returncode:
                raise RuntimeError(result.stdout)
            (fixture / "libelogind.so").symlink_to("libsystemd.so")
            fixtures[version] = fixture

        def check(name, version, features=(), missing=None, overrides=None, manifest=None):
            environment = base_env.copy()
            environment.update({"PKG_CONFIG": "/bin/false", "SYSTEMD_LIB_DIR": str(fixtures[version])})
            environment.update(overrides or {})
            for key in list(environment):
                if environment[key] is None:
                    environment.pop(key)
            command = ["cargo", "check", "--offline", "--no-default-features"]
            if manifest:
                command += ["--manifest-path", str(manifest)]
            else:
                command += ["--locked", "-p", "systemd", "--all-targets"]
            if features:
                command += ["--features", ",".join(features)]
            result = run(command, cwd=ROOT, env=environment)
            if missing:
                passed = result.returncode != 0 and missing in result.stdout
            else:
                passed = result.returncode == 0
            if not passed:
                raise RuntimeError(f"{name}: unexpected result {result.returncode}\n{result.stdout}")
            print(f"{name}: passed", flush=True)

        check("base API rejects missing 233 capability", 232, missing="sd_id128_get_machine_app_specific")
        check("base-only supports 233 exports", 233)
        check("journal without bus supports older exports", 237, ["journal"])
        check("old bus fails during build script", 237, ["bus"], "sd_bus_slot_set_destroy_callback")
        check("default bus supports 239 exports", 239, ["bus", "journal"])
        check("240 API rejects 239 exports", 239, ["systemd_v240"], "sd_id128_get_boot_app_specific")
        check("240 bus API", 240, ["bus", "systemd_v240"])
        check("241 API rejects 240 exports", 240, ["bus", "systemd_v241"], "sd_bus_close_unref")
        check("241 bus API", 241, ["bus", "systemd_v241"])
        check("namespace rejects older exports", 241, ["journal", "systemd_v245"], "sd_journal_open_namespace")
        check("245 API", 245, ["bus", "journal", "systemd_v245"])
        check("session start time rejects 245 exports", 245, ["systemd_v254"], "sd_session_get_start_time")
        check("254 API", 254, ["bus", "journal", "systemd_v254"])

        # Present a numerically new libelogind package that lacks bus slots.
        # Capability detection must use exports, not the .pc version number.
        pkg_dir = directory / "pkgconfig"
        pkg_dir.mkdir()
        (pkg_dir / "libelogind.pc").write_text(
            "Name: libelogind\nDescription: capability fixture\nVersion: 999\n"
            f"Libs: -L{fixtures[237]} -lelogind\n"
        )
        pkg_overrides = {
            "SYSTEMD_LIB_DIR": None, "SYSTEMD_PKG_NAME": "libelogind",
            "PKG_CONFIG": os.environ.get("PKG_CONFIG", "pkg-config"),
            "PKG_CONFIG_LIBDIR": str(pkg_dir),
        }
        check("elogind version does not imply bus support", 237, ["bus"], "sd_bus_slot_set_destroy_callback", pkg_overrides)
        check("elogind base-only succeeds", 237, overrides=pkg_overrides)

        # DSO exports 239; archive exports 254. Respect the requested link kind.
        obj = directory / "exports.o"
        result = run(compiler + ["-fPIC", "-c", str(fixtures[254] / "exports.c"), "-o", str(obj)])
        if result.returncode:
            raise RuntimeError(result.stdout)
        result = run(archiver + ["crs", str(fixtures[239] / "libsystemd.a"), str(obj)])
        if result.returncode:
            raise RuntimeError(result.stdout)
        check("manual static selection", 239, ["bus", "systemd_v254"], overrides={"SYSTEMD_LIBS": "static=systemd"})
        check("manual dynamic selection", 239, ["bus", "systemd_v254"], "sd_session_get_start_time", {"SYSTEMD_LIBS": "dylib=systemd"})
        (pkg_dir / "libsystemd.pc").write_text(
            "Name: libsystemd\nDescription: static capability fixture\nVersion: 999\n"
            f"Libs: -L{fixtures[239]} -lsystemd\n"
        )
        static_overrides = dict(pkg_overrides, SYSTEMD_PKG_NAME="libsystemd", LIBSYSTEMD_STATIC="1")
        check("pkg-config static selection", 239, ["bus", "systemd_v254"], overrides=static_overrides)

        consumer = directory / "consumer"
        (consumer / "src").mkdir(parents=True)
        manifest = consumer / "Cargo.toml"
        manifest.write_text(
            '[package]\nname = "native-api-consumer"\nversion = "0.0.0"\nedition = "2018"\n'
            '[dependencies]\n'
            f'systemd = {{ path = "{ROOT}", default-features = false }}\n'
            f'libsystemd-sys = {{ path = "{ROOT / "libsystemd-sys"}", default-features = false }}\n'
            '[features]\n'
            + "".join(f'{feature} = ["systemd/{feature}", "libsystemd-sys/{feature}"]\n'
                      for feature in ["bus", "journal", "systemd_v240", "systemd_v241", "systemd_v245", "systemd_v254"])
        )
        # Reuse the workspace's tested dependency versions for this external
        # consumer. Its first check only needs to add the consumer package.
        (consumer / "Cargo.lock").write_text((ROOT / "Cargo.lock").read_text())
        probes = [
            ("240 ID", [], "systemd_v240", "from_boot_app_specific", "let _ = systemd::id128::Id128::from_boot_app_specific; let _ = libsystemd_sys::id128::sd_id128_get_boot_app_specific;"),
            ("240 bus", ["bus"], "systemd_v240", "method_call_timeout", "let _ = systemd::bus::BusRef::method_call_timeout; let _ = libsystemd_sys::bus::sd_bus_get_method_call_timeout;"),
            ("241 raw bus", ["bus", "systemd_v240"], "systemd_v241", "sd_bus_close_unref", "let _ = libsystemd_sys::bus::sd_bus_close_unref;"),
            ("245 namespace", ["journal", "systemd_v241"], "systemd_v245", "open_namespace", 'let _ = systemd::journal::OpenOptions::default().open_namespace("test"); let _ = libsystemd_sys::journal::sd_journal_open_namespace;'),
            ("254 session aliases", ["systemd_v245"], "systemd_v254", "session_start_time", 'let _ = systemd::login::get_session_start_time("test"); let _ = systemd::login::session_start_time("test"); let _ = libsystemd_sys::login::sd_session_get_start_time;'),
        ]
        for name, before, feature, missing, source in probes:
            (consumer / "src/main.rs").write_text(f"fn main() {{ {source} }}\n")
            check(f"{name} hidden without feature", 254, before, missing, manifest=manifest)
            check(f"{name} available with feature", 254, before + [feature], manifest=manifest)


if __name__ == "__main__":
    main()
