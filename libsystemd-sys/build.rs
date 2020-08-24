use std::path::Path;

fn main() {
    let name = "systemd";
    let name_upper = name.to_ascii_uppercase();
    let mut be = build_env::BuildEnv::from_env().unwrap();

    let lib_var = format!("{}_LIBS", name_upper);
    let lib_dir_var = format!("{}_LIB_DIR", name_upper);

    let libs = be.var(lib_var);
    let lib_dir = match be.var(lib_dir_var.clone()) {
        Some(lib_dir) => lib_dir,
        None => {
            // No lib_dir specified, use pkg-config
            let ln_vn = format!("{}_PKG_NAME", name_upper);
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
                .unwrap_or_else(|| format!("lib{}", name));

            let library = pkg_config::find_library(&library_name);

            match library {
                Ok(_) => return,
                Err(error) => eprintln!("pkg_config could not find {:?}: {}", library_name, error),
            };

            return;
        }
    };

    if !Path::new(&lib_dir).exists() {
        panic!(
            "{} refers to {:?}, which does not exist",
            lib_dir_var, lib_dir
        );
    }

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );

    match libs {
        Some(libs) => {
            //let libs = libs.expect(&format!("non utf-8 value provided in {}", lib_var));
            for lib in libs.into_string().unwrap().split(':') {
                println!("cargo:rustc-link={}", lib);
            }
        }
        None => {
            println!("cargo:rustc-link={}", name);
        }
    }
}
