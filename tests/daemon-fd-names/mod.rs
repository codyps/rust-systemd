use std::borrow::Cow;
use std::env;
use std::ffi::{CStr, CString, OsString};
use std::fs::File;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::AsRawFd;
use systemd::{daemon, CStrArgument};

pub(super) const CASES: &[&str] = &[
    "names-valid",
    "names-duplicates",
    "names-unknown",
    "names-bytes",
    "names-none",
    "names-other-pid",
    "names-invalid-pid",
    "names-invalid-count",
    "names-short",
    "names-extra",
    "names-bad-fd",
];

fn check_lookup<'a>(
    descriptors: &'a daemon::ListenFdsWithNames,
    name: impl CStrArgument<'a>,
    expected: &[libc::c_int],
) {
    assert_eq!(
        descriptors.iter_named(name).unwrap().collect::<Vec<_>>(),
        expected
    );
}

fn pairs(descriptors: &daemon::ListenFdsWithNames) -> Vec<(libc::c_int, Vec<u8>)> {
    descriptors
        .iter()
        .map(|(fd, name)| (fd, name.to_bytes().to_vec()))
        .collect()
}

/// Run only in the harness-free, single-threaded child process.
pub(super) unsafe fn run(case: &str, unset: bool) {
    // SAFETY: the caller starts this in a fresh process with no other threads
    // or environment-reading signal handlers. Files own all tested raw fds.
    unsafe {
        env::remove_var("LISTEN_PIDFDID");
        env::set_var("LISTEN_PID", std::process::id().to_string());
        env::set_var("LISTEN_FDS", "3");
        env::set_var("LISTEN_FDNAMES", "http:metrics:admin");

        // Simulate the descriptor range supplied by systemd. Clearing CLOEXEC
        // first lets us verify that libsystemd sets it, even on count errors.
        let files: Vec<_> = (0..3).map(|_| File::open("/dev/null").unwrap()).collect();
        for (index, file) in files.iter().enumerate() {
            let fd = file.as_raw_fd();
            assert_eq!(fd, 3 + index as libc::c_int);
            let flags = libc::fcntl(fd, libc::F_GETFD);
            assert!(flags >= 0);
            assert_eq!(libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC), 0);
        }

        let mut expected: Vec<&[u8]> = vec![b"http", b"metrics", b"admin"];
        let mut error = None;
        let mut sets_cloexec = true;
        match case {
            "names-valid" => {}
            "names-duplicates" => {
                env::set_var("LISTEN_FDNAMES", "http:http:admin");
                expected = vec![b"http", b"http", b"admin"];
            }
            "names-unknown" => {
                env::remove_var("LISTEN_FDNAMES");
                expected = vec![b"unknown"; 3];
            }
            "names-bytes" => {
                env::set_var(
                    "LISTEN_FDNAMES",
                    OsString::from_vec(b"http:\xff:admin".to_vec()),
                );
                expected = vec![b"http", b"\xff", b"admin"];
            }
            "names-none" => {
                for key in ["LISTEN_PID", "LISTEN_FDS", "LISTEN_FDNAMES"] {
                    env::remove_var(key);
                }
                expected.clear();
                sets_cloexec = false;
            }
            "names-other-pid" => {
                env::set_var("LISTEN_PID", (std::process::id() + 1).to_string());
                expected.clear();
                sets_cloexec = false;
            }
            "names-invalid-pid" => {
                env::set_var("LISTEN_PID", "invalid");
                error = Some(libc::EINVAL);
                sets_cloexec = false;
            }
            "names-invalid-count" => {
                env::set_var("LISTEN_FDS", "invalid");
                error = Some(libc::EINVAL);
                sets_cloexec = false;
            }
            "names-short" => {
                env::set_var("LISTEN_FDNAMES", "http:metrics");
                error = Some(libc::EINVAL);
            }
            "names-extra" => {
                env::set_var("LISTEN_FDNAMES", "http:metrics:admin:extra");
                error = Some(libc::EINVAL);
            }
            "names-bad-fd" => {
                assert_eq!(libc::fcntl(6, libc::F_GETFD), -1);
                env::set_var("LISTEN_FDS", "4");
                env::set_var("LISTEN_FDNAMES", "http:metrics:admin:extra");
                error = Some(libc::EBADF);
            }
            _ => panic!("unknown named-fd case: {}", case),
        }
        let environment: Vec<_> = [
            "LISTEN_PID",
            "LISTEN_FDS",
            "LISTEN_FDNAMES",
            "LISTEN_PIDFDID",
        ]
        .iter()
        .map(|&key| (key, env::var_os(key)))
        .collect();
        let result = if unset {
            daemon::listen_fds_with_names_unset_environment()
        } else {
            daemon::listen_fds_with_names()
        };
        for (key, before) in environment {
            assert_eq!(env::var_os(key), if unset { None } else { before }, "{key}");
        }
        for file in &files {
            let flags = libc::fcntl(file.as_raw_fd(), libc::F_GETFD);
            assert!(flags >= 0, "the wrapper must not close descriptors");
            assert_eq!(flags & libc::FD_CLOEXEC != 0, sets_cloexec);
        }
        if let Some(error) = error {
            assert_eq!(result.unwrap_err().raw_os_error(), Some(error));
            return;
        }

        let descriptors = result.unwrap();
        let expected: Vec<_> = expected
            .iter()
            .enumerate()
            .map(|(index, &name)| (3 + index as libc::c_int, name.to_vec()))
            .collect();
        assert_eq!(descriptors.len(), expected.len());
        assert_eq!(descriptors.is_empty(), expected.is_empty());
        assert_eq!(descriptors.iter().len(), expected.len());
        assert_eq!(pairs(&descriptors), expected);
        assert_eq!(
            descriptors
                .iter()
                .rev()
                .map(|(fd, _)| fd)
                .collect::<Vec<_>>(),
            expected.iter().rev().map(|(fd, _)| *fd).collect::<Vec<_>>()
        );

        let query = if case == "names-unknown" {
            "unknown"
        } else {
            "http"
        };
        let matches: Vec<_> = expected
            .iter()
            .filter_map(|(fd, name)| (name.as_slice() == query.as_bytes()).then_some(*fd))
            .collect();
        check_lookup(&descriptors, query, &matches);
        check_lookup(&descriptors, query.to_owned(), &matches);
        let c_query = CString::new(query).unwrap();
        check_lookup(&descriptors, c_query.as_c_str(), &matches);
        check_lookup(&descriptors, &c_query, &matches);
        check_lookup(&descriptors, c_query, &matches);
        check_lookup(&descriptors, Cow::<str>::Borrowed(query), &matches);
        check_lookup(
            &descriptors,
            Cow::<CStr>::Owned(CString::new(query).unwrap()),
            &matches,
        );
        assert_eq!(descriptors.iter_named("absent").unwrap().count(), 0);
        assert_eq!(
            descriptors.iter_named("bad\0name").err().unwrap().kind(),
            std::io::ErrorKind::InvalidInput
        );
        if case == "names-bytes" {
            check_lookup(
                &descriptors,
                CStr::from_bytes_with_nul(b"\xff\0").unwrap(),
                &[4],
            );
        }

        let again = daemon::listen_fds_with_names().unwrap();
        assert_eq!(pairs(&again), if unset { vec![] } else { expected });
        drop(again);
        drop(descriptors);
        for file in &files {
            assert!(libc::fcntl(file.as_raw_fd(), libc::F_GETFD) >= 0);
        }
    }
}
