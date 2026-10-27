use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn feature(name: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{name}")).is_some()
}

fn requirements() -> Vec<(&'static str, &'static [&'static str])> {
    let mut groups = vec![(
        "the base API (systemd 233 or equivalent)",
        &["sd_id128_get_machine_app_specific"][..],
    )];
    if feature("BUS") {
        groups.push((
            "feature bus (systemd 239 or equivalent)",
            &[
                "sd_bus_get_n_queued_read",
                "sd_bus_get_n_queued_write",
                "sd_bus_slot_get_floating",
                "sd_bus_slot_set_floating",
                "sd_bus_slot_set_destroy_callback",
                "sd_bus_slot_get_destroy_callback",
                "sd_bus_track_set_destroy_callback",
                "sd_bus_track_get_destroy_callback",
            ],
        ));
    }
    if feature("SYSTEMD_V240") {
        groups.push(("feature systemd_v240", &["sd_id128_get_boot_app_specific"]));
        if feature("BUS") {
            groups.push((
                "features bus + systemd_v240",
                &[
                    "sd_bus_set_close_on_exit",
                    "sd_bus_get_close_on_exit",
                    "sd_bus_set_method_call_timeout",
                    "sd_bus_get_method_call_timeout",
                ],
            ));
        }
    }
    if feature("BUS") && feature("SYSTEMD_V241") {
        groups.push(("features bus + systemd_v241", &["sd_bus_close_unref"]));
    }
    if feature("JOURNAL") && feature("SYSTEMD_V245") {
        groups.push((
            "features journal + systemd_v245",
            &["sd_journal_open_namespace"],
        ));
    }
    if feature("SYSTEMD_V254") {
        groups.push(("feature systemd_v254", &["sd_session_get_start_time"]));
    }
    groups
}

// Mirror pkg-config's static-library selection for the link-only probe, while
// leaving pkg-config responsible for emitting Cargo's actual link metadata.
fn pkg_static_root(name: &str) -> Option<PathBuf> {
    let name = name.to_ascii_uppercase().replace('-', "_");
    let requested = if env::var_os(format!("{name}_STATIC")).is_some() {
        true
    } else if env::var_os(format!("{name}_DYNAMIC")).is_some() {
        false
    } else {
        env::var_os("PKG_CONFIG_ALL_STATIC").is_some()
    };
    let root = env::var_os("PKG_CONFIG_SYSROOT_DIR")
        .or_else(|| env::var_os("SYSROOT"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr"));
    requested.then_some(root)
}

fn push_arg(args: &mut Vec<OsString>, flag: &str, value: impl Into<OsString>) {
    args.push(flag.into());
    args.push(value.into());
}

fn check_symbols(args: &[OsString], target: &str) {
    let groups = requirements();
    let symbols: Vec<_> = groups.iter().flat_map(|(_, names)| names.iter()).collect();
    let declarations: String = symbols
        .iter()
        .map(|name| format!("fn {name}();\n"))
        .collect();
    let pointers: String = symbols
        .iter()
        .map(|name| format!("{name} as unsafe extern \"C\" fn(),\n"))
        .collect();
    let source = format!(
        "extern \"C\" {{ {declarations} }}\nfn main() {{\n\
         let symbols = [{pointers}];\n\
         unsafe {{ std::ptr::read_volatile(&symbols); }}\n}}\n"
    );
    let directory = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let source_path = directory.join("systemd-capabilities.rs");
    fs::write(&source_path, source).expect("write native API link probe");
    // Link for TARGET but never execute the result: this also works for cross
    // builds and validates the exact manually selected library, without headers.
    let mut rustc = Command::new(env::var_os("RUSTC").expect("Cargo must provide RUSTC"));
    rustc
        .arg(&source_path)
        .args([
            "--crate-name",
            "systemd_capabilities",
            "--edition=2018",
            "--target",
            target,
        ])
        .arg("-o")
        .arg(directory.join("systemd-capabilities"));
    if let Some(flags) = env::var_os("CARGO_ENCODED_RUSTFLAGS") {
        rustc.args(
            flags
                .to_str()
                .expect("Rust flags must be UTF-8")
                .split('\x1f')
                .filter(|s| !s.is_empty()),
        );
    }
    if let Some(linker) = env::var_os("RUSTC_LINKER") {
        let mut flag = OsString::from("linker=");
        flag.push(linker);
        rustc.arg("-C").arg(flag);
    }
    let output = rustc
        .args(args)
        .output()
        .expect("run native API link probe");
    if !output.status.success() {
        let enabled = groups
            .iter()
            .map(|(label, _)| *label)
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "native library capability check failed for {enabled}.\n\
             Install a library providing the required symbols, disable bus/journal if unsupported, \
             or disable newer systemd_v* features. This check applies to libelogind and \
             SYSTEMD_LIB_DIR/SYSTEMD_LIBS overrides too.\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn main() {
    let mut be = build_env::BuildEnv::from_env().unwrap();
    let libs = be.var("SYSTEMD_LIBS");
    let mut args = Vec::new();
    if let Some(directory) = be.var("SYSTEMD_LIB_DIR") {
        assert!(
            Path::new(&directory).is_dir(),
            "SYSTEMD_LIB_DIR refers to {:?}, which does not exist or is not a directory",
            directory
        );
        println!(
            "cargo:rustc-link-search=native={}",
            directory.to_string_lossy()
        );
        let mut search = OsString::from("native=");
        search.push(&directory);
        push_arg(&mut args, "-L", search);
        let libs = libs.unwrap_or_else(|| "systemd".into());
        for library in libs
            .to_str()
            .expect("SYSTEMD_LIBS must be UTF-8")
            .split(':')
        {
            println!("cargo:rustc-link-lib={library}");
            push_arg(&mut args, "-l", library);
        }
    } else {
        let package = be
            .var("SYSTEMD_PKG_NAME")
            .unwrap_or_else(|| "libsystemd".into());
        let package = package.to_str().expect("SYSTEMD_PKG_NAME must be UTF-8");
        let library = pkg_config::Config::new()
            .probe(package)
            .unwrap_or_else(|error| panic!("pkg_config could not find {:?}: {}", package, error));
        for path in &library.link_paths {
            let mut search = OsString::from("native=");
            search.push(path);
            push_arg(&mut args, "-L", search);
        }
        let static_root = pkg_static_root(package);
        for name in &library.libs {
            if name.starts_with(':') {
                push_arg(&mut args, "-C", format!("link-arg=-l{name}"));
            } else {
                let is_static = static_root.as_ref().is_some_and(|root| {
                    library.link_paths.iter().any(|path| {
                        !path.starts_with(root) && path.join(format!("lib{name}.a")).is_file()
                    })
                });
                push_arg(
                    &mut args,
                    "-l",
                    if is_static {
                        format!("static={name}")
                    } else {
                        name.clone()
                    },
                );
            }
        }
        for path in &library.link_files {
            let mut flag = OsString::from("link-arg=");
            flag.push(path);
            push_arg(&mut args, "-C", flag);
        }
        for flags in &library.ld_args {
            push_arg(&mut args, "-C", format!("link-arg=-Wl,{}", flags.join(",")));
        }
    }
    check_symbols(&args, be.target());
}
