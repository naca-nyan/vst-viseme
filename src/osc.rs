use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

const PARAMETER_PREFIX: &str = "/avatar/parameters/";

pub fn new_float_message(name: &str, value: f32) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}{name}");
    let clamped_value = value.clamp(0.0f32, 1.0f32);
    let args = vec![OscType::Float(clamped_value)];
    OscMessage { addr, args }
}

pub fn new_note_on_message(name: &str, param_type: &usize, value: f32) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}{name}");
    let arg = match param_type {
        0 => OscType::Bool(true),
        1 => OscType::Int((value * 127.0).round() as i32),
        2 => OscType::Float(value),
        _ => unreachable!(),
    };
    let args = vec![arg];
    OscMessage { addr, args }
}

pub fn new_note_off_message(name: &str, param_type: &usize) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}{name}");
    let arg = match param_type {
        0 => OscType::Bool(false),
        1 => OscType::Int(0),
        2 => OscType::Float(0.0),
        _ => unreachable!(),
    };
    let args = vec![arg];
    OscMessage { addr, args }
}

pub fn new_cc_message(name: &str, param_type: &usize, value: f32) -> OscMessage {
    let addr = format!("{PARAMETER_PREFIX}{name}");
    let arg = match param_type {
        1 => OscType::Int((value * 127.0).round() as i32),
        2 => OscType::Float(value),
        _ => unreachable!(),
    };
    let args = vec![arg];
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
            (OscType::Int(a), OscType::Int(b)) => {
                if a != b {
                    return false;
                }
            }
            (OscType::Bool(a), OscType::Bool(b)) => {
                if a != b {
                    return false;
                }
            }
            (OscType::Float(a), OscType::Float(b)) => {
                if !is_nearly_eq_f32(*a, *b) {
                    return false;
                }
            }
            _ => return false,
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

    pub fn init(&mut self, port: i32) {
        let (tx, rx) = mpsc::sync_channel::<_>(16);
        self.tx = Some(tx);

        thread::spawn(move || {
            let sock = match initialize_send_socket(port) {
                Ok(s) => s,
                Err(e) => {
                    nih_plug::nih_error!("Failed to initialize send OSC socket: {}", e);
                    return;
                }
            };
            while let Ok(msg) = rx.recv() {
                let packet = OscPacket::Message(msg);
                let buf = encoder::encode(&packet).unwrap();
                let _ = sock.send(&buf);
            }
        });
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

pub struct Receiver {
    state: Arc<RwLock<HashMap<String, OscType>>>,
    stop: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
            stop: AtomicBool::default().into(),
            join_handle: None,
        }
    }

    pub fn state(&self) -> Arc<RwLock<HashMap<String, OscType>>> {
        self.state.clone()
    }

    pub fn init(&mut self, receive_port: i32) {
        let state = self.state.clone();
        let stop = self.stop.clone();
        let join_handle = thread::spawn(move || {
            let sock = match initialize_receive_socket(receive_port) {
                Ok(s) => s,
                Err(e) => {
                    nih_plug::nih_error!("Failed to initialize receive OSC socket: {}", e);
                    return;
                }
            };
            let mut buf = [0; decoder::MTU];
            while !stop.load(Ordering::Acquire) {
                if let Ok((len, _)) = sock.recv_from(&mut buf) {
                    if let Ok((_, packet)) = decoder::decode_udp(&buf[..len]) {
                        if let OscPacket::Message(msg) = packet {
                            if let Some(t) = msg.args.first() {
                                state.write().unwrap().insert(msg.addr, t.clone());
                            }
                        }
                    }
                }
            }
        });
        self.join_handle = join_handle.into();
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}

fn initialize_send_socket(port: i32) -> std::io::Result<UdpSocket> {
    let localhost_any = "127.0.0.1:0";
    let target = format!("127.0.0.1:{}", port);
    let sock = UdpSocket::bind(localhost_any)?;
    sock.connect(target)?;
    Ok(sock)
}

fn initialize_receive_socket(port: i32) -> std::io::Result<UdpSocket> {
    let target = format!("127.0.0.1:{}", port);
    let sock = UdpSocket::bind(target)?;
    sock.set_read_timeout(Duration::from_millis(100).into())?;
    Ok(sock)
}
