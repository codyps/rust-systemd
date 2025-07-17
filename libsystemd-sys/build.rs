use std::path::Path;

fn main() {
    let name = "systemd";
    let name_upper = name.to_ascii_uppercase();
    let mut be = build_env::BuildEnv::from_env().unwrap();

    let lib_var = format!("{name_upper}_LIBS");
    let lib_dir_var = format!("{name_upper}_LIB_DIR");

    let libs = be.var(lib_var);
    let lib_dir = match be.var(lib_dir_var.clone()) {
        Some(lib_dir) => lib_dir,
        None => {
            // No lib_dir specified, use pkg-config
            let ln_vn = format!("{name_upper}_PKG_NAME");
            let library_name = be
                .var(&ln_vn)
                .map(|v| {
                    v.into_string().unwrap_or_else(|e| {
                        panic!(
                            "Variable {} could not be converted to a string: {:?}",
                            ln_vn, e
                        )
                    })
                })
                .unwrap_or_else(|| format!("lib{name}"));

            let library = pkg_config::find_library(&library_name);

            match library {
                Ok(_) => {
                    // pkg-config says it has it, so we'll trust it to have done the right thing
                    return;
                }
                Err(error) => {
                    eprintln!("pkg_config could not find {library_name:?}: {error}");
                    std::process::exit(1);
                }
            };
        }
    };

    assert!(
        Path::new(&lib_dir).exists(),
        "{} refers to {:?}, which does not exist",
        lib_dir_var,
        lib_dir
    );

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );

    match libs {
        Some(libs) => {
            //let libs = libs.expect(&format!("non utf-8 value provided in {}", lib_var));
            for lib in libs.into_string().unwrap().split(':') {
                println!("cargo:rustc-link-lib={lib}");
            }
        }
        None => {
            println!("cargo:rustc-link-lib={name}");
        }
    }
}
