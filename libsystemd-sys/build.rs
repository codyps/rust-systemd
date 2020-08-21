use std::path::Path;

fn main() {
    #[cfg(not(feature = "elogind"))]
    let name = "systemd";
    #[cfg(feature = "elogind")]
    let name = "elogind";

    let mut be = build_env::BuildEnv::from_env().unwrap();

    let library_name = format!("lib{}", name);
    let library = pkg_config::find_library(&library_name);

    match library {
        Ok(_) => return,
        Err(error) => eprintln!("pkg_config could not find {}: {}", library_name, error),
    };

    let lib_var = format!("{}_LIBS", name.to_ascii_uppercase());
    let lib_dir_var = format!("{}_LIB_DIR", name.to_ascii_uppercase());

    let libs = be.var(lib_var);
    let lib_dir = match be.var(lib_dir_var.clone()) {
        Some(lib_dir) => lib_dir,
        None => {
            panic!("Environment variable {} is required but is unset", lib_dir_var);
        }
    };

    if !Path::new(&lib_dir).exists() {
        panic!("{} refers to {:?}, which does not exist",
            lib_dir_var, lib_dir);
    }

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );

    match libs {
        Some(libs) => {
            //let libs = libs.expect(&format!("non utf-8 value provided in {}", lib_var));
            for lib in libs.into_string().unwrap().split(":") {
                println!("cargo:rustc-link={}", lib);
            }
        }
        None => {
            println!("cargo:rustc-link={}", name);
        }
    }
}
