#![cfg(target_os = "linux")]

extern crate libc;

use std::{io, slice};

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
            sys::MSG_NODE_DESTROY => Message::NodeDestroy(Handle(msg.destination)),
            sys::MSG_NODE_RELEASE => Message::NodeRelease(Handle(msg.destination)),
            _ => unreachable!()
        })
    }
    pub fn send(&self, destinations: &[u64], buf: &[&[u8]], handles: &[Handle], fds: &[libc::c_int]) -> io::Result<()> {
        let handles = handle_slice_bits(handles);
        self.desc.send(destinations, &buf, handles, fds)
    }
    pub fn transfer_handle(&self, src_handle: Handle, dst: &Peer) -> io::Result<u64> {
        self.desc.handle_transfer(src_handle.0, &dst.desc)
    }
    pub fn release_handle(&self, handle: Handle) -> io::Result<()> {
        self.desc.handle_release(handle.0)
    }
    pub fn destroy_nodes(&self, handles: &[Handle]) -> io::Result<()> {
        let handles = handle_slice_bits(handles);
        self.desc.nodes_destroy(handles)
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Handle(pub u64);

fn handle_slice_bits(handles: &[Handle]) -> &[u64] {
    unsafe {
        slice::from_raw_parts(handles.as_ptr() as *const u64, handles.len())
    }
}

#[derive(Debug)]
pub enum Message<'a> {
    Data(MessageData<'a>),
    NodeDestroy(Handle),
    NodeRelease(Handle)
}

#[derive(Debug)]
pub struct MessageData<'a> {
    peer: &'a Peer,
    msg: sys::msg
}

impl<'a> MessageData<'a> {
    pub fn destination(&self) -> Handle {
        Handle(self.msg.destination)
    }
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
