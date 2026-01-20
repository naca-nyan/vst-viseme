use rosc::{decoder, OscPacket};
use std::net::UdpSocket;

fn receive_loop() {
    let vrc = "127.0.0.1:9000";
    let sock = UdpSocket::bind(vrc).expect("couldn't bind to address");
    loop {
        let mut buf = [0; decoder::MTU];
        let (len, _addr) = sock.recv_from(&mut buf).expect("Didn't receive data");
        let filled_buf = &buf[..len];
        let packet = decoder::decode_udp(&filled_buf).expect("failed to decode");
        match packet.1 {
            OscPacket::Message(msg) => {
                println!("{}: {}", msg.addr, msg.args[0])
            }
            _ => (),
        };
    }
}

fn main() {
    receive_loop();
}
