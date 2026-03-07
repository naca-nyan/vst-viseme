use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};
use std::{net::UdpSocket, thread, time::Duration};

fn receive_loop() {
    let vrc = "127.0.0.1:9000";
    let sock = UdpSocket::bind(vrc).expect("couldn't bind to address");
    loop {
        let mut buf = [0; decoder::MTU];
        let (len, _addr) = sock.recv_from(&mut buf).expect("Didn't receive data");
        let (_, packet) = decoder::decode_udp(&buf[..len]).expect("failed to decode");
        match packet {
            OscPacket::Message(msg) => {
                println!("{}: {}", msg.addr, msg.args[0])
            }
            _ => (),
        };
    }
}

fn send_loop() {
    let localhost_any = "127.0.0.1:0";
    let target = "127.0.0.1:9001";
    let sock = UdpSocket::bind(localhost_any).expect("couldn't bind to address");
    sock.connect(target).expect("couldn't connect to address");
    let mut value = 0.0f32;
    loop {
        let addr = "/avatar/parameters/hoge".to_string();
        let args = vec![OscType::from(value)];
        let msg = OscMessage { addr, args };
        let packet = OscPacket::Message(msg);
        let buf = encoder::encode(&packet).expect("failed to encode");
        sock.send(&buf).expect("failed to send");
        value += 0.1;
        if value > 1.0 {
            value = 0.0
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    thread::spawn(send_loop);
    receive_loop();
}
