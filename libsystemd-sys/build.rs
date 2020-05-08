extern crate pkg_config;
use std::env;

fn main() {
    #[cfg(not(feature = "elogind"))]
    let library = pkg_config::find_library("libsystemd");
    #[cfg(feature = "elogind")]
    let library = pkg_config::find_library("libelogind");

    let error = match library {
        Ok(_) => return,
        Err(error) => error,
    };

    #[cfg(not(feature = "elogind"))]
    let ld_preload_var = "LIBSYSTEMD_LDFLAGS";
    #[cfg(feature = "elogind")]
    let ld_preload_var = "LIBELOGIND_LDFLAGS";

    match env::var(ld_preload_var) {
        Ok(flags) => {
            /* Ideally we'd avoid rustc-flags in favor of rustc-link-{search,lib}, but this should
             * work fine
             */
            println!("cargo:rustc-flags={}", flags);
            println!("cargo:rerun-if-env-changed=LIBSYSTEMD_LDFLAGS");
        }
        Err(_) => {
            eprintln!("{}", error);

            #[cfg(not(feature = "elogind"))]
            let lib_name = "systemd";

            #[cfg(feature = "elogind")]
            let lib_name = "elogind";

            panic!(
                "{} was not found via pkg-config nor via the env var {}",
                lib_name, ld_preload_var
            );
        }
    }
}
