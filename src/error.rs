use std::{borrow::Cow, error, fmt, io};

/// An error from a systemd operation, with its underlying cause and optional context.
///
/// [`Self::kind`] and [`Self::raw_os_error`] preserve the underlying I/O error's classification
/// and errno, including when context is attached. [`Self::operation`] identifies the failed
/// native operation when available; [`Self::argument`] identifies an invalid argument.
/// Argument values are not automatically included in diagnostics.
///
/// The representation is private so additional details can be added without changing this type.
/// The original I/O error is available through [`Self::as_io_error`] and
/// [`std::error::Error::source`]. Custom payloads are available through [`Self::get_ref`].
///
/// # Migrating from `std::io::Error`
///
/// Replacing the previous `std::io::Error` re-export is a breaking API change.
/// `systemd::Error` is a distinct type, and [`crate::Result`] uses it. Existing functions
/// returning `std::io::Result` can continue using `?`, which converts the error:
///
/// ```
/// fn booted() -> std::io::Result<bool> {
///     Ok(systemd::daemon::booted()?)
/// }
/// ```
///
/// Returning the result directly does not perform that conversion:
///
/// ```compile_fail
/// fn booted() -> std::io::Result<bool> {
///     systemd::daemon::booted()
/// }
/// ```
///
/// Direct returns and explicitly typed assignments need conversion, or can switch to
/// `systemd::Result`. Implementations of this crate's traits and callbacks returning
/// `systemd::Result` must also use the new type. Convert explicit I/O errors with `.into()`:
///
/// ```
/// fn reject() -> systemd::Result<()> {
///     Err(std::io::Error::from_raw_os_error(libc::EINVAL).into())
/// }
/// ```
///
/// Conversion into `io::Error` preserves errno for native failures, dropping the added context.
/// For contextual errors without errno, the complete error is retained as a custom payload.
/// Use [`Self::into_io_error_with_context`] to retain all context for any error; the resulting
/// `io::Error` has no direct errno. Converting that wrapper back recovers this error.
#[derive(Debug)]
pub struct Error {
    inner: io::Error,
    operation: Option<Cow<'static, str>>,
    argument: Option<Cow<'static, str>>,
}

impl Error {
    /// Construct an error with an I/O classification and a custom payload.
    pub fn new<E>(kind: io::ErrorKind, error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self {
            inner: io::Error::new(kind, error),
            operation: None,
            argument: None,
        }
    }

    /// Construct an error of kind [`io::ErrorKind::Other`].
    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::new(io::ErrorKind::Other, error)
    }

    /// Construct an error from a native errno.
    pub fn from_raw_os_error(code: i32) -> Self {
        io::Error::from_raw_os_error(code).into()
    }

    /// Capture the current OS error. Call immediately after the failing operation.
    pub fn last_os_error() -> Self {
        io::Error::last_os_error().into()
    }

    /// Return the underlying I/O error's classification.
    pub fn kind(&self) -> io::ErrorKind {
        self.inner.kind()
    }

    /// Return the underlying errno, unaffected by this error's operation or argument context.
    pub fn raw_os_error(&self) -> Option<i32> {
        self.inner.raw_os_error()
    }

    /// Return the operation associated with this error, if known.
    pub fn operation(&self) -> Option<&str> {
        self.operation.as_deref()
    }

    /// Return the argument associated with this error, if known.
    pub fn argument(&self) -> Option<&str> {
        self.argument.as_deref()
    }

    /// Attach or replace the operation name without altering the underlying error.
    /// Static names are borrowed; owned strings are also accepted.
    pub fn with_operation(mut self, operation: impl Into<Cow<'static, str>>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    /// Attach or replace an argument name without altering the underlying error.
    /// This stores the argument's name, rather than its potentially sensitive value.
    pub fn with_argument(mut self, argument: impl Into<Cow<'static, str>>) -> Self {
        self.argument = Some(argument.into());
        self
    }

    /// Borrow the original I/O error, without discarding its native representation.
    pub fn as_io_error(&self) -> &io::Error {
        &self.inner
    }

    /// Borrow the original I/O error's custom payload, if present.
    pub fn get_ref(&self) -> Option<&(dyn error::Error + Send + Sync + 'static)> {
        self.inner.get_ref()
    }

    /// Mutably borrow the original I/O error's custom payload, if present.
    pub fn get_mut(&mut self) -> Option<&mut (dyn error::Error + Send + Sync + 'static)> {
        self.inner.get_mut()
    }

    /// Consume the error and return its original custom payload, discarding added context.
    /// Returns `None` for a native errno error.
    pub fn into_inner(self) -> Option<Box<dyn error::Error + Send + Sync>> {
        self.inner.into_inner()
    }

    /// Convert to an I/O error, preserving errno for native failures.
    ///
    /// Added context is discarded when the underlying error has an errno. Contextual errors
    /// without errno are retained as custom payloads. An error without context converts back
    /// to its original I/O error without introducing a wrapper.
    pub fn into_io_error(self) -> io::Error {
        if self.raw_os_error().is_some() || (self.operation.is_none() && self.argument.is_none()) {
            self.inner
        } else {
            self.into_io_error_with_context()
        }
    }

    /// Retain the complete error inside an I/O error's custom payload.
    ///
    /// The outer error preserves [`Self::kind`], but its `raw_os_error()` returns `None`.
    /// Downcast the payload to `systemd::Error`, or convert it back, to recover context and errno.
    pub fn into_io_error_with_context(self) -> io::Error {
        io::Error::new(self.kind(), self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(operation) = self.operation() {
            write!(formatter, "{operation}: ")?;
        }
        if let Some(argument) = self.argument() {
            write!(formatter, "argument `{argument}`: ")?;
        }
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        // Recover our own context-retaining wrapper rather than nesting another error.
        let kind = error.kind();
        match error.downcast::<Self>() {
            Ok(error) if error.kind() == kind => error,
            // A caller can deliberately reclassify a crate error using io::Error::new.
            Ok(error) => Self::new(kind, error),
            Err(inner) => Self {
                inner,
                operation: None,
                argument: None,
            },
        }
    }
}

impl From<Error> for io::Error {
    fn from(error: Error) -> Self {
        error.into_io_error()
    }
}

impl From<io::ErrorKind> for Error {
    fn from(kind: io::ErrorKind) -> Self {
        io::Error::from(kind).into()
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(error: std::ffi::NulError) -> Self {
        Self::new(io::ErrorKind::InvalidInput, error)
    }
}
