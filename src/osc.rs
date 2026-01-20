use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

pub struct Sender {
    tx: Option<mpsc::SyncSender<f32>>,
}

impl Sender {
    pub fn new() -> Self {
        Self { tx: None }
    }

    pub fn init(&mut self, port: i32) -> bool {
        let (tx, rx) = mpsc::sync_channel::<f32>(16);
        self.tx = Some(tx);

        thread::spawn(move || {
            let sock = match initialize(port) {
                Ok(s) => s,
                Err(e) => {
                    nih_plug::nih_error!("Failed to initialize OSC socket: {}", e);
                    return;
                }
            };
            while let Ok(value) = rx.recv() {
                send_f32(&sock, value);
            }
        });

        true
    }

    pub fn send(&self, value: f32) {
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

fn send_f32(sock: &UdpSocket, v: f32) {
    let packet = OscPacket::Message(OscMessage {
        addr: "/avatar/parameters/Viseme1".into(),
        args: vec![OscType::Float(v)],
    });
    let buf = encoder::encode(&packet).unwrap();
    let _ = sock.send(&buf);
}

#[test]
fn test_send() {
    let mut sender = Sender::new();
    sender.init(9000);
    sender.send(1.0);
}
