#![cfg(target_os = "linux")]

extern crate libc;

use std::io;

pub mod sys;

#[derive(Debug)]
pub struct Peer {
    desc: sys::PeerDesc,
    pool: sys::Pool
}

impl Peer {
    pub fn new() -> io::Result<Peer> {
        let desc = sys::PeerDesc::new()?;
        let pool = desc.map(1 << 30)?;
        Ok(Peer { desc, pool })
    }
    pub fn recv<'a>(&'a self) -> io::Result<Message<'a>> {
        let msg = self.desc.recv(self.pool.len())?;
        Ok(match msg.ty {
            sys::MSG_DATA => Message::Data(MessageData { peer: self, msg }),
            sys::MSG_NODE_DESTROY => Message::NodeDestroy(msg.destination),
            sys::MSG_NODE_RELEASE => Message::NodeRelease(msg.destination),
            _ => unreachable!()
        })
    }
    pub fn send(&self, destinations: &[u64], buf: &[u8], handles: &[u64], fds: &[libc::c_int]) -> io::Result<()> {
        unsafe {
            let iov = libc::iovec {
                iov_base: buf.as_ptr() as *mut libc::c_void,
                iov_len: buf.len() as libc::size_t
            };
            self.desc.send(destinations, &[iov], handles, fds)
        }
    }
    pub fn transfer_handle(&self, src_handle: u64, dst: &Peer) -> io::Result<u64> {
        self.desc.handle_transfer(src_handle, &dst.desc)
    }
}

#[derive(Debug)]
pub enum Message<'a> {
    Data(MessageData<'a>),
    NodeDestroy(u64),
    NodeRelease(u64)
}

#[derive(Debug)]
pub struct MessageData<'a> {
    peer: &'a Peer,
    msg: sys::msg
}

impl<'a> MessageData<'a> {
    pub fn payload(&self) -> &[u8] {
        unsafe { self.msg.payload(&self.peer.pool) }
    }
    pub fn handles(&self) -> &[u64] {
        unsafe { self.msg.handles(&self.peer.pool) }
    }
    pub fn fds(&self) -> &[libc::c_int] {
        unsafe { self.msg.fds(&self.peer.pool) }
    }
}

impl<'a> Drop for MessageData<'a> {
    fn drop(&mut self) {
        self.peer.desc.slice_release(self.msg.offset).expect("couldn't release message slice")
    }
}
