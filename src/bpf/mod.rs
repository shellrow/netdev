#[cfg(not(target_os="windows"))]
mod binding;
#[cfg(not(target_os="windows"))]
pub use self::binding::*;

#[cfg(not(target_os="windows"))]
mod unix;
#[cfg(not(target_os="windows"))]
pub use self::unix::*;

