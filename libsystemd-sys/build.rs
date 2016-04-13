extern crate pkg_config;
use std::{env,fs,ffi};
use std::path::PathBuf;
use std::process::{Command,Stdio};
use std::fs::File;
use std::io::Write;

fn main() {
    match pkg_config::find_library("libsystemd") {
        Ok(_) => return,
        Err(..) => {}
    }

    match env::var("LIBSYSTEMD_LDFLAGS") {
        Ok(flags) => {
            /* Ideally we'd avoid rustc-flags in favor of rustc-link-{search,lib}, but this should
             * work fine
             */
            println!("cargo:rustc-flags={}", flags);
        }
        Err(_) => panic!("systemd was not found via pkg-config nor via the env var LIBSYSTEMD_LDFLAGS"),
    }
}
