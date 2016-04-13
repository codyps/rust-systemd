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

    /* lto like breaking things on travis (which is the primary use of this build script), so we'll
     * disable it using the config-cache
     *
     * Based on http://www.linuxfromscratch.org/lfs/view/systemd/chapter06/systemd.html
     */
    {
        let mut cc = File::create(&build.join("config.cache")).unwrap();
        write!(cc, "cc_cv_CFLAGS__flto=no\n").unwrap();
    }


    /* libsystemd doesn't support being built as static, dynamic required */
    run(Command::new(&src.join("systemd/configure"))
                .current_dir(&build)
                .arg("--config-cache")
                .arg("--enable-kdbus")
                .arg("--disable-tests")
                .arg("--disable-ldconfig")
                .arg("--without-python")
                .arg("--disable-sysusers")
                .arg("--disable-firstboot")
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

    println!("cargo:rustc-link-search=native={}/usr/lib", &dst.display());
    println!("cargo:rustc-link-lib=dylib=systemd");
    println!("cargo:root={}", &dst.display());
    println!("cargo:rustc-link-args=-Wl,-rpath-link={}", &dst.display());
}

fn run(cmd: &mut Command) {
    println!("running: {:?}", cmd);
    assert!(cmd.stdout(Stdio::inherit())
               .stderr(Stdio::inherit())
               .status()
               .unwrap()
               .success());

}
