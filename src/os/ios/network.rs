//! Network.framework helpers for Apple mobile targets.

#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::CStr;
#[cfg(feature = "gateway")]
use std::net::IpAddr;
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use block2::RcBlock;
use dispatch2::{
    DispatchQueue, DispatchQueueGlobalPriority, DispatchRetained, GlobalQueueIdentifier,
};

use crate::interface::types::InterfaceType;
#[cfg(feature = "gateway")]
use crate::net::device::NetworkDevice;

#[derive(Debug, Clone)]
pub(crate) struct NWInterface {
    pub bsd_name: String,
    pub if_type: InterfaceType,
    #[cfg(feature = "gateway")]
    pub index: u32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct NWPathSnapshot {
    pub interfaces: Vec<NWInterface>,
    #[cfg(feature = "gateway")]
    pub satisfied: bool,
    #[cfg(feature = "gateway")]
    pub gateways: Option<NetworkDevice>,
}

impl NWPathSnapshot {
    pub fn interface_map(&self) -> HashMap<String, NWInterface> {
        let mut map = HashMap::with_capacity(self.interfaces.len());
        for iface in &self.interfaces {
            map.insert(iface.bsd_name.clone(), iface.clone());
        }
        map
    }

    #[cfg(feature = "gateway")]
    pub fn first_non_loopback_interface_index(&self) -> Option<u32> {
        if !self.satisfied {
            return None;
        }
        self.interfaces
            .iter()
            .find(|iface| iface.if_type != InterfaceType::Loopback)
            .map(|iface| iface.index)
    }
}

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {}

type nw_path_monitor_t = *mut c_void;
type nw_path_t = *mut c_void;
type nw_interface_t = *mut c_void;
#[cfg(feature = "gateway")]
type nw_endpoint_t = *mut c_void;

unsafe extern "C" {
    fn nw_path_monitor_create() -> nw_path_monitor_t;
    fn nw_path_monitor_set_queue(monitor: nw_path_monitor_t, queue: *mut c_void);
    fn nw_path_monitor_set_update_handler(monitor: nw_path_monitor_t, handler: *mut c_void);
    fn nw_path_monitor_start(monitor: nw_path_monitor_t);
    fn nw_path_monitor_cancel(monitor: nw_path_monitor_t);

    #[cfg(feature = "gateway")]
    fn nw_path_get_status(path: nw_path_t) -> libc::c_uint;
    fn nw_path_enumerate_interfaces(path: nw_path_t, enumerate_block: *mut c_void);
    #[cfg(feature = "gateway")]
    fn nw_path_enumerate_gateways(path: nw_path_t, enumerate_block: *mut c_void);

    fn nw_interface_get_name(interface: nw_interface_t) -> *const c_char;
    fn nw_interface_get_type(interface: nw_interface_t) -> libc::c_uint;
    #[cfg(feature = "gateway")]
    fn nw_interface_get_index(interface: nw_interface_t) -> u32;

    #[cfg(feature = "gateway")]
    fn nw_endpoint_get_address(endpoint: nw_endpoint_t) -> *const libc::sockaddr;

    fn nw_release(obj: *mut c_void);
}

fn map_interface_type(value: libc::c_uint) -> InterfaceType {
    // nw_interface_type_t in Network/interface.h.
    match value {
        1 => InterfaceType::Wireless80211,
        2 => InterfaceType::Wwanpp,
        3 => InterfaceType::Ethernet,
        4 => InterfaceType::Loopback,
        _ => InterfaceType::Unknown,
    }
}

#[cfg(feature = "gateway")]
fn parse_gateway_ip(endpoint: nw_endpoint_t) -> Option<IpAddr> {
    if endpoint.is_null() {
        return None;
    }
    let addr = unsafe { nw_endpoint_get_address(endpoint) };
    if addr.is_null() {
        return None;
    }
    // The endpoint owns the address for the duration of the enumeration callback.
    unsafe { crate::os::unix::sockaddr::SockaddrRef::from_raw(addr, (*addr).sa_len.into()) }
        .map(|address| address.to_ip())
}

fn collect_path_snapshot(path: nw_path_t) -> NWPathSnapshot {
    let mut snapshot = NWPathSnapshot::default();
    let interfaces: Arc<Mutex<Vec<NWInterface>>> = Arc::new(Mutex::new(Vec::new()));
    let interfaces_ref = Arc::clone(&interfaces);
    type InterfaceEnumBlock = dyn Fn(*mut c_void) -> u8 + 'static;
    let interface_block: RcBlock<InterfaceEnumBlock> = RcBlock::new(move |iface_ptr| -> u8 {
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
        let if_type = map_interface_type(unsafe { nw_interface_get_type(iface) });
        #[cfg(feature = "gateway")]
        let index = unsafe { nw_interface_get_index(iface) };

        let nw_interface = NWInterface {
            bsd_name: name,
            if_type,
            #[cfg(feature = "gateway")]
            index,
        };

        interfaces_ref.lock().unwrap().push(nw_interface);

        1
    });

    unsafe {
        nw_path_enumerate_interfaces(
            path,
            RcBlock::<InterfaceEnumBlock>::as_ptr(&interface_block) as *mut c_void,
        );
    }
    snapshot.interfaces = std::mem::take(&mut *interfaces.lock().unwrap());

    #[cfg(feature = "gateway")]
    {
        // nw_path_status_satisfied in Network/path.h.
        snapshot.satisfied = unsafe { nw_path_get_status(path) } == 1;
        let gateways: Arc<Mutex<NetworkDevice>> = Arc::new(Mutex::new(NetworkDevice::new()));
        let gateways_ref = Arc::clone(&gateways);
        type GatewayEnumBlock = dyn Fn(*mut c_void) -> u8 + 'static;
        let gateway_block: RcBlock<GatewayEnumBlock> = RcBlock::new(move |endpoint_ptr| -> u8 {
            if let Some(ip) = parse_gateway_ip(endpoint_ptr as nw_endpoint_t) {
                let mut gateway = gateways_ref.lock().unwrap();
                match ip {
                    IpAddr::V4(ipv4) => {
                        if !gateway.ipv4.contains(&ipv4) {
                            gateway.ipv4.push(ipv4);
                        }
                    }
                    IpAddr::V6(ipv6) => {
                        if !gateway.ipv6.contains(&ipv6) {
                            gateway.ipv6.push(ipv6);
                        }
                    }
                }
            }

            1
        });

        unsafe {
            nw_path_enumerate_gateways(
                path,
                RcBlock::<GatewayEnumBlock>::as_ptr(&gateway_block) as *mut c_void,
            );
        }
        let gateway = gateways.lock().unwrap();
        if !gateway.ipv4.is_empty() || !gateway.ipv6.is_empty() {
            snapshot.gateways = Some(gateway.clone());
        }
    }
    snapshot
}

type UpdateHandler = dyn Fn(*mut c_void) + 'static;

/// Capture a single snapshot of the current default Network.framework path.
pub(crate) fn current_path_snapshot() -> Option<NWPathSnapshot> {
    let shared: Arc<(Mutex<Option<NWPathSnapshot>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let shared_ref = Arc::clone(&shared);

    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
        DispatchQueueGlobalPriority::Default,
    ));

    let update_block: RcBlock<UpdateHandler> = RcBlock::new(move |path_ptr: *mut c_void| {
        if path_ptr.is_null() {
            return;
        }

        let snapshot = collect_path_snapshot(path_ptr as nw_path_t);
        let (lock, cv) = &*shared_ref;
        let mut guard = lock.lock().unwrap();
        if guard.is_none() {
            *guard = Some(snapshot);
            cv.notify_one();
        }
    });

    unsafe {
        let monitor = nw_path_monitor_create();
        if monitor.is_null() {
            return None;
        }

        let queue_ptr = DispatchRetained::<DispatchQueue>::as_ptr(&queue)
            .as_ptr()
            .cast::<c_void>();
        let update_ptr = RcBlock::<UpdateHandler>::as_ptr(&update_block) as *mut c_void;

        nw_path_monitor_set_queue(monitor, queue_ptr);
        nw_path_monitor_set_update_handler(monitor, update_ptr);
        nw_path_monitor_start(monitor);

        let (lock, cv) = &*shared;
        let guard = lock.lock().unwrap();
        let (mut guard, _) = cv
            .wait_timeout_while(guard, Duration::from_millis(800), |value| value.is_none())
            .unwrap();
        let snapshot = guard.take();
        drop(guard);

        nw_path_monitor_cancel(monitor);
        nw_release(monitor);

        snapshot
    }
}
