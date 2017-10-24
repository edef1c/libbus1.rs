extern crate bus1;
extern crate libc;

fn main() {
    let peer1 = bus1::Peer::new().expect("couldn't create peer1");
    let peer2 = bus1::Peer::new().expect("couldn't create peer2");

    let handle2 = peer1.transfer_handle(0, &peer2).expect("couldn't transfer handle");
    peer2.send(&[handle2], b"hello, world", &[], &[]).expect("couldn't send message");
    match peer1.recv().expect("couldn't receive message") {
        bus1::Message::Data(msg) => println!("{:?}", msg.payload()),
        _ => unreachable!()
    };
}
