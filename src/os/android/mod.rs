pub mod arp;
pub mod flags;
pub mod interface;
pub mod mtu;
pub mod netlink;
#[cfg(feature = "gateway")]
pub mod procfs;
pub mod state;
pub mod sysfs;

use once_cell::sync::OnceCell;

pub fn get_libc_ifaddrs() -> Option<(
    unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int,
    unsafe extern "C" fn(*mut libc::ifaddrs),
)> {
    match (get_getifaddrs(), get_freeifaddrs()) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}

fn load_symbol<T>(sym: &'static str) -> Option<T> {
    const LIB_NAME: &str = "libc.so";

    match dlopen2::raw::Library::open(LIB_NAME) {
        Ok(lib) => match unsafe { lib.symbol::<T>(sym) } {
            Ok(val) => Some(val),
            Err(err) => {
                eprintln!("failed to load symbol {} from {}: {:?}", sym, LIB_NAME, err);
                None
            }
        },
        Err(err) => {
            eprintln!("failed to load {}: {:?}", LIB_NAME, err);
            None
        }
    }
}

fn get_getifaddrs() -> Option<unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int> {
    static INSTANCE: OnceCell<
        Option<unsafe extern "C" fn(*mut *mut libc::ifaddrs) -> libc::c_int>,
    > = OnceCell::new();

    *INSTANCE.get_or_init(|| load_symbol("getifaddrs"))
}

fn get_freeifaddrs() -> Option<unsafe extern "C" fn(*mut libc::ifaddrs)> {
    static INSTANCE: OnceCell<Option<unsafe extern "C" fn(*mut libc::ifaddrs)>> = OnceCell::new();

    *INSTANCE.get_or_init(|| load_symbol("freeifaddrs"))
}
