use windows::Win32::NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO, IP_ADDR_STRING};
use std::convert::TryInto;
use std::mem;

pub const ERROR_BUFFER_OVERFLOW: u32 = 111;
pub const NO_ERROR: u32 = 0;

// Get network interface information using the IP Helper API
// TODO: Make more rusty way...
// Reference: https://docs.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersinfo
pub fn get_interfaces() {
    let mut out_buf_len : u32 = mem::size_of::<IP_ADAPTER_INFO>().try_into().unwrap();
    let mut raw_adaptor_mem: Vec<u8> = Vec::with_capacity(out_buf_len  as usize);
    let mut p_adaptor: *mut IP_ADAPTER_INFO;
    let mut res = unsafe { GetAdaptersInfo(raw_adaptor_mem.as_mut_ptr() as *mut IP_ADAPTER_INFO, &mut out_buf_len ) };
    // Make an initial call to GetAdaptersInfo to get the necessary size into the out_buf_len variable
    if res == ERROR_BUFFER_OVERFLOW {
		raw_adaptor_mem = Vec::with_capacity(out_buf_len as usize);
		unsafe {
			res = GetAdaptersInfo(raw_adaptor_mem.as_mut_ptr() as *mut IP_ADAPTER_INFO, &mut out_buf_len);
		}
	}
    if res != NO_ERROR {
        //TODO
		println!("failed to get adapters info");
        // for test
		std::process::exit(1);
	}
    //Enumerate all adapters
	p_adaptor = unsafe { mem::transmute(&raw_adaptor_mem) };
    while p_adaptor as u64 != 0 {
        unsafe {
			let adapter = *p_adaptor;
            let adapter_name = String::from_utf8_lossy(&adapter.AdapterName);
            let adapter_desc = String::from_utf8_lossy(&adapter.Description);
            let mac_addr = adapter.Address.to_vec();
            println!("{} {} {} {} {:?}", adapter.Index, adapter.ComboIndex, adapter_name, adapter_desc, mac_addr);
            //Enumerate all IPs
            let mut p_ip_addr: *mut IP_ADDR_STRING;
            p_ip_addr = mem::transmute(&(*p_adaptor).IpAddressList);
            while p_ip_addr as u64 != 0 {
                let ip_addr_string = *p_ip_addr;
                let ip_addr = String::from_utf8_lossy(&ip_addr_string.IpAddress.String);
                println!("{}", ip_addr);
                p_ip_addr = (*p_ip_addr).Next;
            }
            //Enumerate all gateways
            let mut p_gateway_addr: *mut IP_ADDR_STRING;
            p_gateway_addr = mem::transmute(&(*p_adaptor).GatewayList);
            while p_gateway_addr as u64 != 0 {
                let gateway_addr_string = *p_gateway_addr;
                let gateway_addr = String::from_utf8_lossy(&gateway_addr_string.IpAddress.String);
                println!("{}", gateway_addr);
                p_gateway_addr = (*p_gateway_addr).Next;
            }
            //TODO
		}
        unsafe { p_adaptor = (*p_adaptor).Next; }
    }
}

#[cfg(test)]
mod tests {
    use crate::os::windows;
    #[test]
    fn list_nw_interfaces() {
        windows::get_interfaces();
    }
}
