use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

use crate::Addresses;

pub struct OscValue {
    pub addr: Addresses,
    pub value: f32,
}

pub struct Sender {
    tx: Option<mpsc::SyncSender<OscValue>>,
}

impl Sender {
    pub fn new() -> Self {
        Self { tx: None }
    }

    pub fn init(&mut self, port: i32) -> bool {
        let (tx, rx) = mpsc::sync_channel::<OscValue>(16);
        self.tx = Some(tx);

        thread::spawn(move || {
            let sock = match initialize(port) {
                Ok(s) => s,
                Err(e) => {
                    nih_plug::nih_error!("Failed to initialize OSC socket: {}", e);
                    return;
                }
            };
            while let Ok(osc_value) = rx.recv() {
                send_f32(&sock, osc_value);
            }
        });

        true
    }

    pub fn send(&self, value: OscValue) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(value);
        }
    }
}

fn initialize(port: i32) -> std::io::Result<UdpSocket> {
    let localhost_any = "127.0.0.1:0";
    let target = format!("127.0.0.1:{}", port);
    let sock = UdpSocket::bind(localhost_any)?;
    sock.connect(target)?;
    Ok(sock)
}

fn send_f32(sock: &UdpSocket, v: OscValue) {
    let addr = match v.addr {
        Addresses::Viseme1 => "/avatar/parameters/Viseme1",
        Addresses::Viseme2 => "/avatar/parameters/Viseme2",
        Addresses::Viseme3 => "/avatar/parameters/Viseme3",
        Addresses::Viseme4 => "/avatar/parameters/Viseme4",
        Addresses::Viseme5 => "/avatar/parameters/Viseme5",
    }
    .into();
    let args = vec![OscType::Float(v.value)];
    let packet = OscPacket::Message(OscMessage { addr, args });
    let buf = encoder::encode(&packet).unwrap();
    let _ = sock.send(&buf);
}

#[test]
fn test_send() {
    let mut sender = Sender::new();
    sender.init(9000);
    let value = OscValue {
        addr: Addresses::Viseme1,
        value: 1.0,
    };
    sender.send(value);
}
