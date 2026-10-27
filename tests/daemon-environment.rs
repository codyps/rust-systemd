//! Run environment-mutating tests in fresh processes without the threaded test harness.

use std::env;
use std::os::unix::net::UnixDatagram;
use std::process::Command;
use std::time::Duration;
use systemd::daemon;

fn check_environment(names: &[&str], unset: bool) {
    for name in names {
        assert_eq!(env::var_os(name).is_none(), unset, "{name}");
    }
}

fn child(case: &str, unset: bool) {
    // SAFETY: this binary has no test harness and never spawns threads. All
    // environment access and libsystemd calls happen sequentially in main.
    unsafe {
        match case {
            "listen" | "listen-error" => {
                env::set_var(
                    "LISTEN_PID",
                    if case == "listen-error" {
                        "invalid".to_owned()
                    } else {
                        (std::process::id() + 1).to_string()
                    },
                );
                env::set_var("LISTEN_FDS", "1");
                env::set_var("LISTEN_FDNAMES", "socket");
                let result = if unset {
                    daemon::listen_fds_unset_environment()
                } else {
                    daemon::listen_fds()
                };
                if case == "listen-error" {
                    assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EINVAL));
                } else {
                    assert!(result.unwrap().is_empty());
                }
                check_environment(&["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"], unset);
            }
            "watchdog" | "watchdog-error" => {
                env::set_var(
                    "WATCHDOG_USEC",
                    if case == "watchdog-error" {
                        "invalid"
                    } else {
                        "1000000"
                    },
                );
                env::set_var("WATCHDOG_PID", std::process::id().to_string());
                let result = if unset {
                    daemon::watchdog_enabled_unset_environment()
                } else {
                    daemon::watchdog_enabled()
                };
                if case == "watchdog-error" {
                    assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::EINVAL));
                } else {
                    assert_eq!(result.unwrap(), 1_000_000);
                }
                check_environment(&["WATCHDOG_USEC", "WATCHDOG_PID"], unset);
            }
            "notify" | "pid-notify" | "pid-notify-with-fds" => {
                let path =
                    env::temp_dir().join(format!("rust-systemd-notify-{}", std::process::id()));
                let socket = UnixDatagram::bind(&path).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                env::set_var("NOTIFY_SOCKET", &path);
                let state = [(daemon::STATE_READY, "1")];
                let pid = std::process::id() as libc::pid_t;
                let result = match (case, unset) {
                    ("notify", false) => daemon::notify(state.iter()),
                    ("notify", true) => daemon::notify_unset_environment(state.iter()),
                    ("pid-notify", false) => daemon::pid_notify(pid, state.iter()),
                    ("pid-notify", true) => daemon::pid_notify_unset_environment(pid, state.iter()),
                    ("pid-notify-with-fds", false) => {
                        daemon::pid_notify_with_fds(pid, state.iter(), &[])
                    }
                    ("pid-notify-with-fds", true) => {
                        daemon::pid_notify_with_fds_unset_environment(pid, state.iter(), &[])
                    }
                    _ => unreachable!(),
                };
                assert!(result.unwrap());
                check_environment(&["NOTIFY_SOCKET"], unset);
                let mut buffer = [0; 64];
                let length = socket.recv(&mut buffer).unwrap();
                assert_eq!(&buffer[..length], b"READY=1");
                std::fs::remove_file(path).unwrap();
            }
            _ => panic!("unknown case: {}", case),
        }
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--daemon-environment-child") {
        child(&args[2], args[3] == "unset");
        return;
    }
    for case in [
        "listen",
        "listen-error",
        "watchdog",
        "watchdog-error",
        "notify",
        "pid-notify",
        "pid-notify-with-fds",
    ] {
        for mode in ["keep", "unset"] {
            let output = Command::new(env::current_exe().unwrap())
                .args(["--daemon-environment-child", case, mode])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{case}/{mode}: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            println!("{case}/{mode}: passed");
        }
    }
}
