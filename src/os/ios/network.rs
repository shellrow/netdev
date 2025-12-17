//! Minimal Network.framework interface enumeration

#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use block2::RcBlock;
use dispatch2::{
    DispatchQueue, DispatchQueueGlobalPriority, DispatchRetained, GlobalQueueIdentifier,
};

use crate::interface::types::InterfaceType;

#[derive(Debug, Clone)]
pub(crate) struct NWInterface {
    pub bsd_name: String,
    pub if_type: InterfaceType,
    #[allow(dead_code)]
    pub index: u32,
}

// Link to Network.framework
#[link(name = "Network", kind = "framework")]
unsafe extern "C" {}

// Network.framework (minimal) FFI types
type nw_path_monitor_t = *mut c_void;
type nw_path_t = *mut c_void;
type nw_interface_t = *mut c_void;

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
enum nw_interface_type_t {
    other = 0,
    wifi = 1,
    cellular = 2,
    wired = 3,
    loopback = 4,
}

unsafe extern "C" {
    fn nw_path_monitor_create() -> nw_path_monitor_t;
    fn nw_path_monitor_set_queue(monitor: nw_path_monitor_t, queue: *mut c_void);
    fn nw_path_monitor_set_update_handler(monitor: nw_path_monitor_t, handler: *mut c_void);
    fn nw_path_monitor_start(monitor: nw_path_monitor_t);
    fn nw_path_monitor_cancel(monitor: nw_path_monitor_t);

    fn nw_path_enumerate_interfaces(path: nw_path_t, enumerate_block: *mut c_void);

    fn nw_interface_get_name(interface: nw_interface_t) -> *const c_char;
    fn nw_interface_get_type(interface: nw_interface_t) -> nw_interface_type_t;
    fn nw_interface_get_index(interface: nw_interface_t) -> u32;

    fn nw_release(obj: *mut c_void);
}

fn map_type(t: nw_interface_type_t) -> InterfaceType {
    match t {
        nw_interface_type_t::wifi => InterfaceType::Wireless80211,
        nw_interface_type_t::cellular => InterfaceType::Wwanpp,
        nw_interface_type_t::wired => InterfaceType::Ethernet,
        nw_interface_type_t::loopback => InterfaceType::Loopback,
        nw_interface_type_t::other => InterfaceType::Unknown,
    }
}

type EnumBlock = dyn Fn(*mut c_void) -> u8 + 'static;

/// Enumerate interfaces from Network.framework (NWPath/NWInterface)
pub fn nfw_interfaces() -> Vec<NWInterface> {
    let shared: Arc<(Mutex<Option<Vec<NWInterface>>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let shared2 = Arc::clone(&shared);

    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
        DispatchQueueGlobalPriority::Default,
    ));

    type UpdateHandler = dyn Fn(*mut c_void) + 'static;

    let blk: RcBlock<UpdateHandler> = RcBlock::new(move |path_ptr: *mut c_void| {
        let path = path_ptr as nw_path_t;

        let acc: Arc<Mutex<Vec<NWInterface>>> = Arc::new(Mutex::new(Vec::new()));
        let acc2 = Arc::clone(&acc);

        let enum_blk: RcBlock<EnumBlock> = RcBlock::new(move |iface_ptr: *mut c_void| -> u8 {
            let iface = iface_ptr as nw_interface_t;
            if iface.is_null() {
                return 1;
            }

            let name_ptr = unsafe { nw_interface_get_name(iface) };
            if name_ptr.is_null() {
                return 1;
            }

            let name = unsafe { CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned();

            let ty = unsafe { nw_interface_get_type(iface) };
            let idx = unsafe { nw_interface_get_index(iface) };

            acc2.lock().unwrap().push(NWInterface {
                bsd_name: name,
                if_type: map_type(ty),
                index: idx,
            });

            1
        });

        let enum_ptr: *mut c_void = RcBlock::<EnumBlock>::as_ptr(&enum_blk) as *mut c_void;
        unsafe { nw_path_enumerate_interfaces(path, enum_ptr) };

        // Take only the first callback result and notify
        let v = std::mem::take(&mut *acc.lock().unwrap());
        let (lock, cv) = &*shared2;
        let mut g = lock.lock().unwrap();
        if g.is_none() {
            *g = Some(v);
            cv.notify_one();
        }

        // note: enum_blk lives until the end of this closure
    });

    unsafe {
        let monitor = nw_path_monitor_create();
        if monitor.is_null() {
            return Vec::new();
        }

        let q_nn = DispatchRetained::<DispatchQueue>::as_ptr(&queue);
        let q_ptr: *mut c_void = q_nn.as_ptr() as *mut c_void;

        let blk_ptr: *mut c_void = RcBlock::<UpdateHandler>::as_ptr(&blk) as *mut c_void;

        nw_path_monitor_set_queue(monitor, q_ptr);
        nw_path_monitor_set_update_handler(monitor, blk_ptr);
        nw_path_monitor_start(monitor);

        let (lock, cv) = &*shared;

        let guard = lock.lock().unwrap();
        if guard.is_none() {
            let _ = cv.wait_timeout(guard, Duration::from_millis(500)).unwrap();
        }

        nw_path_monitor_cancel(monitor);
        nw_release(monitor as *mut c_void);
    }

    shared.0.lock().unwrap().take().unwrap_or_default()
}

pub fn nw_interface_map() -> HashMap<String, NWInterface> {
    let mut map = HashMap::new();
    let ifaces = nfw_interfaces();
    for iface in ifaces {
        map.insert(iface.bsd_name.clone(), iface);
    }
    map
}
