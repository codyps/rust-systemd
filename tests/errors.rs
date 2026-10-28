use std::error::Error as _;
use std::io;
use systemd::{CStrArgument, Error};

#[test]
fn native_errors_keep_errno_and_kind_when_context_is_added() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
    let original = io::Error::from_raw_os_error(libc::EACCES);
    let original_message = original.to_string();
    let error = Error::from(original)
        .with_operation(String::from("open resource"))
        .with_argument("path");
    assert_eq!(error.raw_os_error(), Some(libc::EACCES));
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.operation(), Some("open resource"));
    assert_eq!(error.argument(), Some("path"));
    assert_eq!(
        error.to_string(),
        format!("open resource: argument `path`: {original_message}")
    );
    assert_eq!(error.as_io_error().raw_os_error(), Some(libc::EACCES));
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::EACCES)
    );
    assert!(error.get_ref().is_none());

    // The default compatibility conversion preserves errno, discarding added context.
    let converted: io::Error = error.into();
    assert_eq!(converted.raw_os_error(), Some(libc::EACCES));
    assert_eq!(converted.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(converted.to_string(), original_message);
}

#[test]
fn explicit_context_wrapper_round_trips_errno_and_details() {
    let error = Error::from_raw_os_error(libc::EIO)
        .with_operation("read resource")
        .with_argument("fd");
    let converted = error.into_io_error_with_context();
    assert_eq!(
        converted.kind(),
        io::Error::from_raw_os_error(libc::EIO).kind()
    );
    assert_eq!(converted.raw_os_error(), None);
    let payload = converted
        .get_ref()
        .unwrap()
        .downcast_ref::<Error>()
        .unwrap();
    assert_eq!(payload.raw_os_error(), Some(libc::EIO));
    assert_eq!(payload.operation(), Some("read resource"));

    let recovered = Error::from(converted);
    assert_eq!(recovered.raw_os_error(), Some(libc::EIO));
    assert_eq!(recovered.operation(), Some("read resource"));
    assert_eq!(recovered.argument(), Some("fd"));
}

#[test]
fn contextual_non_native_errors_retain_the_payload_when_converted() {
    let error = Error::other(std::fmt::Error)
        .with_operation("format record")
        .with_argument("formatter");
    let converted: io::Error = error.into();
    assert_eq!(converted.kind(), io::ErrorKind::Other);
    assert!(converted.get_ref().unwrap().is::<Error>());
    let recovered = Error::from(converted);
    assert_eq!(recovered.operation(), Some("format record"));
    assert_eq!(recovered.argument(), Some("formatter"));
    assert!(recovered.get_ref().unwrap().is::<std::fmt::Error>());
}

#[test]
fn uncontextualized_io_errors_keep_their_original_payload() {
    let converted: io::Error = Error::from(io::Error::other(std::fmt::Error)).into();
    assert!(converted.get_ref().unwrap().is::<std::fmt::Error>());
    let classified = Error::from(io::ErrorKind::UnexpectedEof);
    assert_eq!(classified.kind(), io::ErrorKind::UnexpectedEof);
    assert_eq!(classified.raw_os_error(), None);
}

#[test]
fn explicit_reclassification_preserves_the_outer_error_kind() {
    for error in [
        Error::new(
            io::ErrorKind::InvalidData,
            Error::from_raw_os_error(libc::EINVAL),
        ),
        io::Error::new(
            io::ErrorKind::InvalidData,
            Error::from_raw_os_error(libc::EINVAL),
        )
        .into(),
    ] {
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.raw_os_error(), None);
        let cause = error.get_ref().unwrap().downcast_ref::<Error>().unwrap();
        assert_eq!(cause.raw_os_error(), Some(libc::EINVAL));
    }
}

#[test]
fn question_mark_converts_in_both_directions() {
    fn io_result() -> io::Result<()> {
        systemd::ffi_result(-libc::EACCES)?;
        Ok(())
    }
    fn systemd_result() -> systemd::Result<()> {
        Err(io::Error::from_raw_os_error(libc::ENOENT))?;
        Ok(())
    }
    assert_eq!(io_result().unwrap_err().raw_os_error(), Some(libc::EACCES));
    assert_eq!(
        systemd_result().unwrap_err().raw_os_error(),
        Some(libc::ENOENT)
    );
    assert_eq!(systemd::ffi_result(7).unwrap(), 7);
}

#[test]
fn custom_payloads_remain_accessible() {
    #[derive(Debug)]
    struct Cause(u8);
    impl std::fmt::Display for Cause {
        fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(out, "cause {}", self.0)
        }
    }
    impl std::error::Error for Cause {}
    let mut error = Error::other(Cause(1)).with_operation("custom operation");
    error.get_mut().unwrap().downcast_mut::<Cause>().unwrap().0 = 2;
    assert_eq!(
        error.get_ref().unwrap().downcast_ref::<Cause>().unwrap().0,
        2
    );
    assert_eq!(
        error.into_inner().unwrap().downcast::<Cause>().unwrap().0,
        2
    );
}

#[test]
fn string_conversion_preserves_the_validation_cause() {
    let error = "a\0b".try_into_cstr().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(error.operation(), Some("convert C string argument"));
    assert_eq!(error.argument(), Some("value"));
    let cause = error
        .get_ref()
        .unwrap()
        .downcast_ref::<std::ffi::NulError>()
        .unwrap();
    assert_eq!(cause.nul_position(), 1);

    let error: Error = std::ffi::CString::new("a\0b").unwrap_err().into();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.get_ref().unwrap().is::<std::ffi::NulError>());
}

#[test]
fn native_failure_identifies_the_operation() {
    let error = systemd::id128::Id128::from_cstr("not-an-id").unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    assert_eq!(error.operation(), Some("sd_id128_from_string"));
    assert!(error.to_string().starts_with("sd_id128_from_string: "));
}

#[test]
#[cfg(feature = "bus")]
fn checked_name_traits_use_the_crate_error() {
    fn assert_error_types<T>()
    where
        T: std::str::FromStr<Err = Error> + std::convert::TryFrom<String, Error = Error>,
    {
    }
    assert_error_types::<systemd::bus::ObjectPathBuf>();
    assert_error_types::<systemd::bus::BusNameBuf>();
    assert_error_types::<systemd::bus::InterfaceNameBuf>();
    assert_error_types::<systemd::bus::MemberNameBuf>();
    let error = "invalid"
        .parse::<systemd::bus::ObjectPathBuf>()
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(error.operation(), Some("ObjectPath::from_bytes"));
    assert_eq!(error.argument(), Some("value"));
}

#[test]
#[cfg(feature = "bus")]
fn dbus_conversion_keeps_error_mapping_and_context() {
    let native = systemd::bus::Error::from(io::Error::from_raw_os_error(libc::EACCES));
    let error = systemd::bus::Error::from(
        Error::from_raw_os_error(libc::EACCES)
            .with_operation("read resource")
            .with_argument("fd"),
    );
    assert_eq!(error.name(), native.name());
    let message: &str = error.message().unwrap().as_ref();
    assert!(message.starts_with("read resource: argument `fd`: "));

    let error = systemd::bus::Error::from(Error::other("bad\0input 100%"));
    let message: &str = error.message().unwrap().as_ref();
    assert_eq!(message, "bad\\0input 100%");
}
