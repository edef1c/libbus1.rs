use libc;
use std::{io, mem, ptr, slice};
use std::os::unix::io::{FromRawFd, AsRawFd, IntoRawFd};

#[repr(C)]
#[derive(Debug)]
pub struct cmd_peer_reset {
    flags: u64,
    peer_flags: u64,
    max_slices: u32,
    max_handles: u32,
    max_inflight_bytes: u32,
    max_inflight_fds: u32
}

#[repr(C)]
struct cmd_handle_transfer {
    flags: u64,
    src_handle: u64,
    dst_fd: u64,
    dst_handle: u64
}

#[repr(C)]
struct cmd_send {
    flags: u64,
    ptr_destinations: u64,
    ptr_errors: u64,
    n_destinations: u64,
    ptr_vecs: u64,
    n_vecs: u64,
    ptr_handles: u64,
    n_handles: u64,
    ptr_fds: u64,
    n_fds: u64
}

#[repr(C)]
struct cmd_recv {
    flags: u64,
    max_offset: u64,
    msg: msg
}

#[derive(Debug)]
#[repr(C)]
pub struct msg {
    pub ty: u64,
    pub flags: u64,
    pub destination: u64,
    pub offset: u64,
    pub n_bytes: u64,
    pub n_handles: u64,
    pub n_fds: u64
}

#[repr(C)]
struct cmd_node_destroy {
    flags: u64,
    ptr_nodes: u64,
    n_nodes: u64
}

const CMD_PEER_DISCONNECT: libc::c_ulong = 0xc0089600;
const CMD_PEER_QUERY:      libc::c_ulong = 0xc0209601;
const CMD_PEER_RESET:      libc::c_ulong = 0xc0209602;
const CMD_HANDLE_RELEASE:  libc::c_ulong = 0xc0089610;
const CMD_HANDLE_TRANSFER: libc::c_ulong = 0xc0209611;
const CMD_NODES_DESTROY:   libc::c_ulong = 0xc0189620;
const CMD_SLICE_RELEASE:   libc::c_ulong = 0xc0089630;
const CMD_SEND:            libc::c_ulong = 0xc0509640;
const CMD_RECV:            libc::c_ulong = 0xc0489650;

pub const MSG_NONE:         u64 = 0;
pub const MSG_DATA:         u64 = 1;
pub const MSG_NODE_DESTROY: u64 = 2;
pub const MSG_NODE_RELEASE: u64 = 3;

pub const HANDLE_FLAG_MANAGED: u64 = 1 << 0;
pub const HANDLE_FLAG_REMOTE:  u64 = 1 << 1;

#[derive(Debug)]
pub struct PeerDesc {
    lower: libc::c_int
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SendFlag {
    Seed,
    Continue
}

impl FromRawFd for PeerDesc {
    unsafe fn from_raw_fd(lower: libc::c_int) -> PeerDesc {
        PeerDesc { lower }
    }
}

impl AsRawFd for PeerDesc {
    fn as_raw_fd(&self) -> libc::c_int {
        self.lower
    }
}

impl IntoRawFd for PeerDesc {
    fn into_raw_fd(self) -> libc::c_int {
        let lower = self.lower;
        mem::forget(self);
        lower
    }
}

impl PeerDesc {
    pub fn new() -> io::Result<PeerDesc> {
        unsafe {
            match libc::open("/dev/bus1\0".as_ptr() as *const libc::c_char, libc::O_RDWR) {
                -1 => Err(io::Error::last_os_error()),
                fd => Ok(PeerDesc::from_raw_fd(fd))
            }
        }
    }
    pub fn map(&self, length: usize) -> io::Result<Pool> {
        Pool::new(self.lower, length)
    }
    pub fn peer_disconnect(&self) -> io::Result<()> {
        unsafe {
            match libc::ioctl(self.lower, CMD_PEER_DISCONNECT) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn peer_query(&self) -> io::Result<cmd_peer_reset> {
        unsafe {
            let mut arg: cmd_peer_reset = mem::uninitialized();
            arg.flags = 0;
            match libc::ioctl(self.lower, CMD_PEER_QUERY, &mut arg) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(arg)
            }
        }
    }
    pub fn peer_reset(&self, args: cmd_peer_reset) -> io::Result<()> {
        unsafe {
            match libc::ioctl(self.lower, CMD_PEER_RESET, &args) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn handle_release(&self, handle: u64) -> io::Result<()> {
        unsafe {
            match libc::ioctl(self.lower, CMD_HANDLE_RELEASE, handle) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn handle_transfer(&self, src_handle: u64, dst: &PeerDesc) -> io::Result<u64> {
        unsafe {
            let mut arg = cmd_handle_transfer {
                flags: 0,
                src_handle,
                dst_fd: dst.lower as u64,
                dst_handle: !0
            };
            match libc::ioctl(self.lower, CMD_HANDLE_TRANSFER, &mut arg) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(arg.dst_handle)
            }
        }
    }
    pub fn nodes_destroy(&self, node_handles: &[u64]) -> io::Result<()> {
        unsafe {
            let arg = cmd_node_destroy {
                flags: 0,
                ptr_nodes: node_handles.as_ptr() as usize as u64,
                n_nodes: node_handles.len() as u64
            };
            match libc::ioctl(self.lower, CMD_NODES_DESTROY, &arg) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn slice_release(&self, pool_offset: u64) -> io::Result<()> {
        unsafe {
            match libc::ioctl(self.lower, CMD_SLICE_RELEASE, &pool_offset) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn send(&self, destinations: &[u64], payload: &[&[u8]], handles: &[u64], fds: &[libc::c_int]) -> io::Result<()> {
        unsafe {
            // assert that they're the same size, at compile time
            mem::transmute::<&[u8], libc::iovec>(&[]);
            let arg = cmd_send {
                flags: 0,
                ptr_destinations: destinations.as_ptr() as usize as u64,
                ptr_errors: 0,
                n_destinations: destinations.len() as u64,
                ptr_vecs: payload.as_ptr() as usize as u64,
                n_vecs: payload.len() as u64,
                ptr_handles: handles.as_ptr() as usize as u64,
                n_handles: handles.len() as u64,
                ptr_fds: fds.as_ptr() as usize as u64,
                n_fds: fds.len() as u64
            };
            match libc::ioctl(self.lower, CMD_SEND, &arg) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(())
            }
        }
    }
    pub fn recv(&self, max_offset: usize) -> io::Result<msg> {
        unsafe {
            let mut arg = cmd_recv {
                flags: 0,
                max_offset: max_offset as u64,
                msg: mem::zeroed()
            };
            match libc::ioctl(self.lower, CMD_RECV, &mut arg) {
                -1 => Err(io::Error::last_os_error()),
                _  => Ok(arg.msg)
            }
        }
    }
}

impl Drop for PeerDesc {
    fn drop(&mut self) {
        unsafe { libc::close(self.lower); }
    }
}

#[derive(Debug)]
pub struct Pool {
    ptr: *const u8,
    len: usize
}

impl Pool {
    fn new(fd: libc::c_int, len: usize) -> io::Result<Pool> {
        let ptr = unsafe { libc::mmap(ptr::null_mut(), len, libc::PROT_READ, libc::MAP_SHARED, fd, 0) };
        if ptr == libc::MAP_FAILED {
            Err(io::Error::last_os_error())
        } else {
            Ok(Pool { ptr: ptr as *const u8, len })
        }
    }
    pub fn len(&self) -> usize { self.len }
}

impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len); }
    }
}

impl msg {
    unsafe fn ptrs(&self, pool: &Pool) -> (*const u8, *const u64, *const libc::c_int) {
        let payload = pool.ptr.offset(self.offset as isize);
        let handles = payload.offset((self.n_bytes as isize + 7) & !7) as *const u64;
        let fds = handles.offset(self.n_handles as isize) as *const libc::c_int;
        (payload, handles, fds)
    }
    pub unsafe fn payload<'a>(&self, pool: &'a Pool) -> &'a [u8] {
        let (ptr, _, _) = self.ptrs(pool);
        slice::from_raw_parts(ptr, self.n_bytes as usize)
    }
    pub unsafe fn handles<'a>(&self, pool: &'a Pool) -> &'a [u64] {
        let (_, ptr, _) = self.ptrs(pool);
        slice::from_raw_parts(ptr, self.n_handles as usize)
    }
    pub unsafe fn fds<'a>(&self, pool: &'a Pool) -> &'a [libc::c_int] {
        let (_, _, ptr) = self.ptrs(pool);
        slice::from_raw_parts(ptr, self.n_fds as usize)
    }
}
