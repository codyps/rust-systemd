#![feature(env, path, fs, process, os)]

extern crate pkg_config;
use std::{env,fs,ffi};
use std::path::PathBuf;
use std::process::{Command,Stdio};

fn main() {
    match pkg_config::find_library("libsystemd") {
        Ok(_) => return,
        Err(..) => {}
    }

    match env::var("SYSTEMD_BUILD") {
        Ok(_) => build_systemd(),
        Err(_) => panic!("systemd was not found & building it was not enabled with 'SYSTEMD_BUILD=1'"),
    }
}

fn build_systemd() {
    let src = PathBuf::from(&env::current_dir().unwrap());
    let dst = PathBuf::from(&env::var("OUT_DIR").unwrap());
    let build = dst.join("build");

    let _ = fs::create_dir(&build);

    /* XXX: will running this in src potentially in parallel with other builders cause issues? */
    run(Command::new(&src.join("systemd/autogen.sh"))
                .current_dir(&src));

    /* libsystemd doesn't support being built as static, dynamic required */
    run(Command::new(&src.join("systemd/configure"))
                .current_dir(&build)
                .arg("--enable-kdbus")
                .arg("--disable-tests")
                .arg("--disable-ldconfig")
                .arg("--disable-manpages"));

    let mut jobs : ffi::OsString = From::from("-j");
    jobs.push(&env::var("NUM_JOBS").unwrap());
    run(Command::new("make")
                .current_dir(&build)
                .arg(&jobs));

    run(Command::new("make")
                .current_dir(&build)
                .env("DESTDIR", &dst)
                .arg(&jobs)
                .arg("install"));

    println!("cargo:rustc-flags=-L {:?}/usr/lib -l systemd:dynamic", &dst);
    println!("cargo:root={:?}", &dst);

    /* WTF do we need include paths for? */
    println!("cargo:include={:?}/include", &dst);
}

fn run(cmd: &mut Command) {
    println!("running: {:?}", cmd);
    assert!(cmd.stdout(Stdio::inherit())
               .stderr(Stdio::inherit())
               .status()
               .unwrap()
               .success());

}
