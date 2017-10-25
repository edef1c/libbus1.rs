extern crate bus1;
extern crate libc;

use std::str;

fn main() {
    let peer1 = bus1::Peer::new().expect("couldn't create peer1");
    let peer2 = bus1::Peer::new().expect("couldn't create peer2");

    let handle2 = peer1.transfer_handle(bus1::Handle(0), &peer2).expect("couldn't transfer handle");
    peer2.send(&[handle2], "hello, world".as_bytes(), &[], &[]).expect("couldn't send message");
    match peer1.recv().expect("couldn't receive message") {
        bus1::Message::Data(msg) => println!("{}", str::from_utf8(msg.payload()).unwrap()),
        _ => unreachable!()
    };
}
