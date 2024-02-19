use std::io;
use std::time::Duration;

use crate::bpf;
use crate::interface::Interface;

pub type EtherType = u16;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChannelType {
    Layer2,
    Layer3(EtherType),
}

#[non_exhaustive]
pub enum Channel {
    Ethernet(Box<dyn FrameSender>, Box<dyn FrameReceiver>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FanoutOption {
    pub group_id: u16,
    pub defrag: bool,
    pub rollover: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    pub write_buffer_size: usize,
    pub read_buffer_size: usize,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub channel_type: ChannelType,
    pub bpf_fd_attempts: usize,
    pub promiscuous: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            read_timeout: None,
            write_timeout: None,
            channel_type: ChannelType::Layer2,
            bpf_fd_attempts: 1000,
            promiscuous: true,
        }
    }
}

#[inline]
pub fn channel(interface_name: String, configuration: Config) -> io::Result<Channel> {
    bpf::channel(interface_name, (&configuration).into())
}

pub trait FrameSender: Send {
    fn build_and_send(
        &mut self,
        num_packets: usize,
        packet_size: usize,
        func: &mut dyn FnMut(&mut [u8]),
    ) -> Option<io::Result<()>>;

    fn send_to(&mut self, packet: &[u8], dst: Option<Interface>) -> Option<io::Result<()>>;
}

pub trait FrameReceiver: Send {
    fn next(&mut self) -> io::Result<&[u8]>;
}
