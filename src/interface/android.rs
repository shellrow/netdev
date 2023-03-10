use once_cell::sync::OnceCell;

fn load_symbol<T>(sym: &'static str) -> Option<T> {
    const LIB_NAME: &str = "libc.so";

    println!("loading symbol: {} from {}", sym, LIB_NAME);
    match dlopen::raw::Library::open(LIB_NAME) {
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

pub unsafe fn getifaddrs(ifap: *mut *mut libc::ifaddrs) -> libc::c_int {
    // Android is complicated

    // API 24+ contains the getifaddrs and freeifaddrs functions but the NDK doesn't
    // expose those functions in ifaddrs.h when the minimum supported SDK is lower than 24
    // and therefore we need to load them manually.
    if let Some(dyn_getifaddrs) = get_getifaddrs() {
        return dyn_getifaddrs(ifap);
    }

    // If API < 24 (or we can't load libc for some other reason), we fallback to using netlink
    netlink_getifaddrs(ifap)
}

pub unsafe fn freeifaddrs(ifa: *mut libc::ifaddrs) {
    if let Some(dyn_freeifaddrs) = get_freeifaddrs() {
        return dyn_freeifaddrs(ifa);
    }

    netlink_freeifaddrs(ifa)
}

unsafe fn netlink_getifaddrs(ifap: *mut *mut libc::ifaddrs) -> libc::c_int {
    todo!()
}

unsafe fn netlink_freeifaddrs(ifa: *mut libc::ifaddrs) {
    todo!()
}
