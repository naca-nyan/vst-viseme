use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

use crate::Address;

#[derive(Clone)]
pub struct OscValue {
    addr: Address,
    value: f32,
}

impl OscValue {
    pub fn new(addr: Address, value: f32) -> Self {
        Self {
            addr: addr,
            value: value.clamp(0.0f32, 1.0f32),
        }
    }
}

impl PartialEq for OscValue {
    fn eq(&self, other: &Self) -> bool {
        const THRESHOLD: f32 = 0.001;
        self.addr == other.addr && (self.value - other.value).abs() < THRESHOLD
    }
}

pub struct Sender {
    tx: Option<mpsc::SyncSender<OscValue>>,
    previous_value: Option<OscValue>,
}

impl Sender {
    pub fn new() -> Self {
        Self {
            tx: None,
            previous_value: None,
        }
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

    pub fn send(&mut self, value: OscValue) {
        if let Some(prev) = &self.previous_value {
            if *prev == value {
                return;
            }
        }
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(value.clone());
        }
        self.previous_value = value.into();
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
    let addr = v.addr.as_str().into();
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
        addr: Address::Viseme1,
        value: 1.0,
    };
    sender.send(value);
}
