use super::binding;
use crate::interface::Interface;
use crate::socket::{FrameReceiver, FrameSender};

use std::collections::VecDeque;
use std::ffi::CString;
use std::io;
use std::mem;
use std::ptr;
use std::sync::Arc;
use std::time::Duration;

static ETHERNET_HEADER_SIZE: usize = 14;

pub type CSocket = libc::c_int;
pub type TvUsecType = libc::c_int;

unsafe fn close(sock: CSocket) {
    let _ = libc::close(sock);
}

fn duration_to_timespec(dur: Duration) -> libc::timespec {
    libc::timespec {
        tv_sec: dur.as_secs() as libc::time_t,
        tv_nsec: (dur.subsec_nanos() as TvUsecType).into(),
    }
}

pub struct FileDesc {
    pub fd: CSocket,
}

impl Drop for FileDesc {
    fn drop(&mut self) {
        unsafe {
            close(self.fd);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Config {
    pub write_buffer_size: usize,
    pub read_buffer_size: usize,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub bpf_fd_attempts: usize,
}

impl<'a> From<&'a crate::socket::Config> for Config {
    fn from(config: &crate::socket::Config) -> Config {
        Config {
            write_buffer_size: config.write_buffer_size,
            read_buffer_size: config.read_buffer_size,
            bpf_fd_attempts: config.bpf_fd_attempts,
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            bpf_fd_attempts: 1000,
            read_timeout: None,
            write_timeout: None,
        }
    }
}

#[inline]
pub fn channel(interface_name: String, config: Config) -> io::Result<crate::socket::Channel> {
    #[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
    fn get_fd(_attempts: usize) -> libc::c_int {
        let c_file_name = CString::new(&b"/dev/bpf"[..]).unwrap();
        unsafe { libc::open(c_file_name.as_ptr(), libc::O_RDWR, 0) }
    }

    #[allow(temporary_cstring_as_ptr)]
    #[cfg(any(target_os = "openbsd", target_os = "macos", target_os = "ios"))]
    fn get_fd(attempts: usize) -> libc::c_int {
        for i in 0..attempts {
            let fd = unsafe {
                let file_name = format!("/dev/bpf{}", i);
                let c_file_name = CString::new(file_name.as_bytes()).unwrap();
                libc::open(c_file_name.as_ptr(), libc::O_RDWR, 0)
            };
            if fd != -1 {
                return fd;
            }
        }

        -1
    }

    #[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
    fn set_feedback(fd: libc::c_int) -> io::Result<()> {
        if unsafe { binding::ioctl(fd, binding::BIOCFEEDBACK, &1) } == -1 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }
        Ok(())
    }

    #[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "ios"))]
    fn set_feedback(_fd: libc::c_int) -> io::Result<()> {
        Ok(())
    }

    let fd = get_fd(config.bpf_fd_attempts);
    if fd == -1 {
        return Err(io::Error::last_os_error());
    }
    let mut iface: binding::ifreq = unsafe { mem::zeroed() };
    for (i, c) in interface_name.bytes().enumerate() {
        iface.ifr_name[i] = c as i8;
    }

    let buflen = config.read_buffer_size as libc::c_uint;

    if unsafe { binding::ioctl(fd, binding::BIOCSBLEN, &buflen) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    if unsafe { binding::ioctl(fd, binding::BIOCSETIF, &iface) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    if unsafe { binding::ioctl(fd, binding::BIOCIMMEDIATE, &1) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    let mut dlt: libc::c_uint = 0;
    if unsafe { binding::ioctl(fd, binding::BIOCGDLT, &mut dlt) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(err);
    }

    let mut loopback = false;
    let mut allocated_read_buffer_size = config.read_buffer_size;

    if dlt == binding::DLT_NULL {
        loopback = true;

        allocated_read_buffer_size += ETHERNET_HEADER_SIZE;

        if let Err(e) = set_feedback(fd) {
            return Err(e);
        }
    } else {
        if unsafe { binding::ioctl(fd, binding::BIOCSHDRCMPLT, &1) } == -1 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }
    }

    if unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
        let err = io::Error::last_os_error();
        unsafe {
            close(fd);
        }
        return Err(err);
    }

    let fd = Arc::new(FileDesc { fd: fd });
    let mut sender = Box::new(FrameSenderImpl {
        fd: fd.clone(),
        fd_set: unsafe { mem::zeroed() },
        write_buffer: vec![0; config.write_buffer_size],
        loopback: loopback,
        timeout: config.write_timeout.map(|to| duration_to_timespec(to)),
    });
    unsafe {
        libc::FD_ZERO(&mut sender.fd_set as *mut libc::fd_set);
        libc::FD_SET(fd.fd, &mut sender.fd_set as *mut libc::fd_set);
    }
    let mut receiver = Box::new(FrameReceiverImpl {
        fd: fd.clone(),
        fd_set: unsafe { mem::zeroed() },
        read_buffer: vec![0; allocated_read_buffer_size],
        loopback: loopback,
        timeout: config.read_timeout.map(|to| duration_to_timespec(to)),
        packets: VecDeque::with_capacity(allocated_read_buffer_size / 64),
    });
    unsafe {
        libc::FD_ZERO(&mut receiver.fd_set as *mut libc::fd_set);
        libc::FD_SET(fd.fd, &mut receiver.fd_set as *mut libc::fd_set);
    }
    Ok(crate::socket::Channel::Ethernet(sender, receiver))
}

struct FrameSenderImpl {
    fd: Arc<FileDesc>,
    fd_set: libc::fd_set,
    write_buffer: Vec<u8>,
    loopback: bool,
    timeout: Option<libc::timespec>,
}

impl FrameSender for FrameSenderImpl {
    #[inline]
    fn build_and_send(
        &mut self,
        num_packets: usize,
        packet_size: usize,
        func: &mut dyn FnMut(&mut [u8]),
    ) -> Option<io::Result<()>> {
        let len = num_packets * packet_size;
        if len >= self.write_buffer.len() {
            None
        } else {
            let offset = if self.loopback {
                ETHERNET_HEADER_SIZE
            } else {
                0
            };
            for chunk in self.write_buffer[..len].chunks_mut(packet_size) {
                func(chunk);
                let ret = unsafe {
                    libc::FD_SET(self.fd.fd, &mut self.fd_set as *mut libc::fd_set);
                    libc::pselect(
                        self.fd.fd + 1,
                        ptr::null_mut(),
                        &mut self.fd_set as *mut libc::fd_set,
                        ptr::null_mut(),
                        self.timeout
                            .as_ref()
                            .map(|to| to as *const libc::timespec)
                            .unwrap_or(ptr::null()),
                        ptr::null(),
                    )
                };
                if ret == -1 {
                    return Some(Err(io::Error::last_os_error()));
                } else if ret == 0 {
                    return Some(Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out")));
                } else {
                    match unsafe {
                        libc::write(
                            self.fd.fd,
                            chunk.as_ptr().offset(offset as isize) as *const libc::c_void,
                            (chunk.len() - offset) as libc::size_t,
                        )
                    } {
                        len if len == -1 => return Some(Err(io::Error::last_os_error())),
                        _ => (),
                    }
                }
            }
            Some(Ok(()))
        }
    }

    #[inline]
    fn send_to(&mut self, packet: &[u8], _dst: Option<Interface>) -> Option<io::Result<()>> {
        let offset = if self.loopback {
            ETHERNET_HEADER_SIZE
        } else {
            0
        };
        let ret = unsafe {
            libc::FD_SET(self.fd.fd, &mut self.fd_set as *mut libc::fd_set);
            libc::pselect(
                self.fd.fd + 1,
                ptr::null_mut(),
                &mut self.fd_set as *mut libc::fd_set,
                ptr::null_mut(),
                self.timeout
                    .as_ref()
                    .map(|to| to as *const libc::timespec)
                    .unwrap_or(ptr::null()),
                ptr::null(),
            )
        };
        if ret == -1 {
            return Some(Err(io::Error::last_os_error()));
        } else if ret == 0 {
            return Some(Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out")));
        } else {
            match unsafe {
                libc::write(
                    self.fd.fd,
                    packet.as_ptr().offset(offset as isize) as *const libc::c_void,
                    (packet.len() - offset) as libc::size_t,
                )
            } {
                len if len == -1 => Some(Err(io::Error::last_os_error())),
                _ => Some(Ok(())),
            }
        }
    }
}

struct FrameReceiverImpl {
    fd: Arc<FileDesc>,
    fd_set: libc::fd_set,
    read_buffer: Vec<u8>,
    loopback: bool,
    timeout: Option<libc::timespec>,
    packets: VecDeque<(usize, usize)>,
}

impl FrameReceiver for FrameReceiverImpl {
    fn next(&mut self) -> io::Result<&[u8]> {
        let (header_size, buffer_offset) = if self.loopback {
            (4, ETHERNET_HEADER_SIZE)
        } else {
            (0, 0)
        };
        if self.packets.is_empty() {
            let buffer = &mut self.read_buffer[buffer_offset..];
            let ret = unsafe {
                libc::FD_SET(self.fd.fd, &mut self.fd_set as *mut libc::fd_set);
                libc::pselect(
                    self.fd.fd + 1,
                    &mut self.fd_set as *mut libc::fd_set,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    self.timeout
                        .as_ref()
                        .map(|to| to as *const libc::timespec)
                        .unwrap_or(ptr::null()),
                    ptr::null(),
                )
            };
            if ret == -1 {
                return Err(io::Error::last_os_error());
            } else if ret == 0 {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "Timed out"));
            } else {
                let buflen = match unsafe {
                    libc::read(
                        self.fd.fd,
                        buffer.as_ptr() as *mut libc::c_void,
                        buffer.len() as libc::size_t,
                    )
                } {
                    len if len > 0 => len,
                    _ => return Err(io::Error::last_os_error()),
                };
                let mut ptr = buffer.as_mut_ptr();
                let end = unsafe { buffer.as_ptr().offset(buflen as isize) };
                while (ptr as *const u8) < end {
                    unsafe {
                        let packet: *const binding::bpf_hdr = mem::transmute(ptr);
                        let start =
                            ptr as isize + (*packet).bh_hdrlen as isize - buffer.as_ptr() as isize;
                        self.packets.push_back((
                            start as usize + header_size,
                            (*packet).bh_caplen as usize - header_size,
                        ));
                        let offset = (*packet).bh_hdrlen as isize + (*packet).bh_caplen as isize;
                        ptr = ptr.offset(binding::BPF_WORDALIGN(offset));
                    }
                }
            }
        }
        let (start, mut len) = self.packets.pop_front().unwrap();
        len += buffer_offset;
        for i in (&mut self.read_buffer[start..start + buffer_offset]).iter_mut() {
            *i = 0;
        }
        Ok(&self.read_buffer[start..start + len])
    }
}
