//! Helper macros. Note that these are relatively new and may change in a later version.
//!
//! The idea is to use `itry` for internal server operations which can't be recovered from, and
//! `iexpect` for validating user input. Note that this kind of usage is completely non-normative.
//! Feedback about actual usability and usage is appreciated.

/// Like `try!()`, but wraps the error value in `IronError`. To be used in
/// request handlers.
///
/// The second (optional) parameter is any [modifier](modifiers/index.html).
/// The default modifier is `status::InternalServerError`.
///
///
/// ```ignore
/// let f = itry!(fs::File::create("foo.txt"), status::BadRequest);
/// let f = itry!(fs::File::create("foo.txt"), (status::NotFound, "Not Found"));
/// let f = itry!(fs::File::create("foo.txt"));  // HTTP 500
/// ```
///
#[macro_export]
macro_rules! itry {
    ($result:expr) => {
        itry!($result, $crate::status::InternalServerError)
    };

    ($result:expr, $modifier:expr) => {
        match $result {
            Ok(val) => val,
            Err(err) => return Err($crate::IronError::new(err, $modifier)),
        }
    };
}

/// Unwrap the given `Option` or return a `Ok(Response::new())` with the given
/// modifier. The default modifier is `status::BadRequest`.
#[macro_export]
macro_rules! iexpect {
    ($option:expr) => {
        iexpect!($option, $crate::status::BadRequest)
    };
    ($option:expr, $modifier:expr) => {
        match $option {
            Some(x) => x,
            None => return Ok($crate::response::Response::with($modifier)),
        }
    };
}
