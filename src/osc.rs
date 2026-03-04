use rosc::{encoder, OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

const PARAMETER_PREFIX: &str = "/avatar/parameters/";

pub fn new_float_message(name: &str, value: f32) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}{name}");
    let clamped_value = value.clamp(0.0f32, 1.0f32);
    let args = vec![OscType::Float(clamped_value)];
    OscMessage { addr, args }
}

pub fn new_note_message(note: u8, value: bool) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}Note{note}");
    let args = vec![OscType::Bool(value)];
    OscMessage { addr, args }
}

fn is_nearly_eq_f32(a: f32, b: f32) -> bool {
    const THRESHOLD: f32 = 0.001;
    (a - b).abs() < THRESHOLD
}

fn is_nearly_eq(a: &OscMessage, b: &OscMessage) -> bool {
    if a.addr != b.addr || a.args.len() != b.args.len() {
        return false;
    }
    for (a, b) in a.args.iter().zip(b.args.iter()) {
        match (a, b) {
            (OscType::Int(a), OscType::Int(b)) if a != b => return false,
            (OscType::Bool(a), OscType::Bool(b)) if a != b => return false,
            (OscType::Float(a), OscType::Float(b)) if !is_nearly_eq_f32(*a, *b) => return false,
            _ => (),
        }
    }
    return true;
}

pub struct Sender {
    tx: Option<mpsc::SyncSender<OscMessage>>,
    state: HashMap<String, OscMessage>,
}

impl Sender {
    pub fn new() -> Self {
        Self {
            tx: None,
            state: HashMap::new(),
        }
    }

    pub fn init(&mut self, port: i32) -> bool {
        let (tx, rx) = mpsc::sync_channel::<_>(16);
        self.tx = Some(tx);

        thread::spawn(move || {
            let sock = match initialize(port) {
                Ok(s) => s,
                Err(e) => {
                    nih_plug::nih_error!("Failed to initialize OSC socket: {}", e);
                    return;
                }
            };
            while let Ok(msg) = rx.recv() {
                let packet = OscPacket::Message(msg);
                let buf = encoder::encode(&packet).unwrap();
                let _ = sock.send(&buf);
            }
        });

        true
    }

    pub fn send(&mut self, value: OscMessage) {
        if let Some(prev) = self.state.get(&value.addr) {
            if is_nearly_eq(prev, &value) {
                return;
            }
        }
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(value.clone());
        }
        self.state.insert(value.addr.clone(), value);
    }
}

fn initialize(port: i32) -> std::io::Result<UdpSocket> {
    let localhost_any = "127.0.0.1:0";
    let target = format!("127.0.0.1:{}", port);
    let sock = UdpSocket::bind(localhost_any)?;
    sock.connect(target)?;
    Ok(sock)
}
