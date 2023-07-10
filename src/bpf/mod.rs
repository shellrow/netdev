#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
mod binding;
#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub use self::binding::*;

#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
mod unix;
#[cfg(any(
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub use self::unix::*;

#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios"
))]
mod shared;
#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "ios"
))]
pub use self::shared::*;