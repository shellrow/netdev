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

mod netlink {
    //! Netlink based getifaddrs.
    //!
    //! Based on the logic found in https://git.musl-libc.org/cgit/musl/tree/src/network/getifaddrs.c

    use netlink_packet_core::{
        NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_DUMP, NLM_F_REQUEST,
    };
    use netlink_packet_route::{
        rtnl::address::nlas::Nla as AddressNla, rtnl::link::nlas::Nla as LinkNla, AddressMessage,
        LinkMessage, RtnlMessage,
    };
    use netlink_sys::{protocols::NETLINK_ROUTE, Socket};
    use std::net::{Ipv4Addr, Ipv6Addr};

    use crate::interface::{Interface, InterfaceType, Ipv4Net, Ipv6Net, MacAddr};

    pub fn unix_interfaces() -> Vec<Interface> {
        let socket = Socket::new(NETLINK_ROUTE).unwrap();

        let mut ifaces = Vec::new();
        enumerate_netlink(
            &socket,
            RtnlMessage::GetLink(LinkMessage::default()),
            &mut ifaces,
        );
        enumerate_netlink(
            &socket,
            RtnlMessage::GetAddress(AddressMessage::default()),
            &mut ifaces,
        );

        ifaces
    }

    fn enumerate_netlink(socket: &Socket, msg: RtnlMessage, ifaces: &mut Vec<Interface>) {
        let mut packet = NetlinkMessage::new(NetlinkHeader::default(), NetlinkPayload::from(msg));
        packet.header.flags = NLM_F_DUMP | NLM_F_REQUEST;
        packet.header.sequence_number = 1;
        packet.finalize();

        let mut buf = vec![0; packet.header.length as usize];

        // TODO: gracefully handle error
        assert!(buf.len() == packet.buffer_len());
        packet.serialize(&mut buf[..]);

        socket.send(&buf[..], 0).unwrap();

        let mut receive_buffer = vec![0; 4096];
        let mut offset = 0;

        loop {
            let size = socket.recv(&mut &mut receive_buffer[..], 0).unwrap();

            loop {
                let bytes = &receive_buffer[offset..];
                let rx_packet: NetlinkMessage<RtnlMessage> =
                    NetlinkMessage::deserialize(bytes).unwrap();

                match rx_packet.payload {
                    NetlinkPayload::Done => {
                        return;
                    }
                    NetlinkPayload::Error(err) => {
                        eprintln!("Error: {:?}", err);
                        return;
                    }
                    NetlinkPayload::InnerMessage(msg) => {
                        match msg {
                            RtnlMessage::NewLink(link_msg) => {
                                let mut interface: Interface = Interface {
                                    index: link_msg.header.index,
                                    name: String::new(),
                                    friendly_name: None,
                                    description: None,
                                    if_type: InterfaceType::try_from(
                                        link_msg.header.link_layer_type as u32,
                                    )
                                    .unwrap_or(InterfaceType::Unknown),
                                    mac_addr: None,
                                    ipv4: Vec::new(),
                                    ipv6: Vec::new(),
                                    flags: link_msg.header.flags,
                                    transmit_speed: None,
                                    receive_speed: None,
                                    gateway: None,
                                };

                                for nla in link_msg.nlas {
                                    match nla {
                                        LinkNla::IfName(name) => {
                                            interface.name = name;
                                        }
                                        LinkNla::Address(addr) => {
                                            match addr.len() {
                                                6 => {
                                                    interface.mac_addr = Some(MacAddr::new(
                                                        addr.try_into().unwrap(),
                                                    ));
                                                }
                                                4 => {
                                                    let ip = Ipv4Addr::from(
                                                        <[u8; 4]>::try_from(addr).unwrap(),
                                                    );
                                                    interface.ipv4.push(Ipv4Net::new_with_netmask(
                                                        ip,
                                                        Ipv4Addr::UNSPECIFIED,
                                                    ));
                                                }
                                                _ => {
                                                    // unclear what these would be
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                ifaces.push(interface);
                            }
                            RtnlMessage::NewAddress(addr_msg) => {
                                println!("NewAddress: {:?}", addr_msg);
                                if let Some(interface) =
                                    ifaces.iter_mut().find(|i| i.index == addr_msg.header.index)
                                {
                                    for nla in addr_msg.nlas {
                                        match nla {
                                            AddressNla::Address(addr) => match addr.len() {
                                                4 => {
                                                    let ip = Ipv4Addr::from(
                                                        <[u8; 4]>::try_from(addr).unwrap(),
                                                    );
                                                    interface.ipv4.push(Ipv4Net::new(
                                                        ip,
                                                        addr_msg.header.prefix_len,
                                                    ));
                                                }
                                                16 => {
                                                    let ip = Ipv6Addr::from(
                                                        <[u8; 16]>::try_from(addr).unwrap(),
                                                    );
                                                    interface.ipv6.push(Ipv6Net::new(
                                                        ip,
                                                        addr_msg.header.prefix_len,
                                                    ));
                                                }
                                                _ => {
                                                    // what else?
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "found unknown interface with index: {}",
                                        addr_msg.header.index
                                    );
                                }
                            }
                            _ => {
                                // not expecting other messages
                            }
                        }
                    }
                    _ => {}
                }
                offset += rx_packet.header.length as usize;
                if offset == size || rx_packet.header.length == 0 {
                    offset = 0;
                    break;
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_netlink_ifaddrs() {
            let interfaces = unix_interfaces();
            dbg!(&interfaces);
            assert!(!interfaces.is_empty());
        }
    }
}
