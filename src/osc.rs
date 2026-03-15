use std::{
    collections::HashMap,
    io,
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, RwLock,
    },
    thread,
    time::Duration,
};

use nih_plug::nih_error;
use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};

use crate::utils::{decode_unicode_escapes, encode_unicode_escapes};

const PARAMETER_PREFIX: &str = "/avatar/parameters/";

fn name_to_address(name: &str) -> String {
    let name = encode_unicode_escapes(name);
    format!("{PARAMETER_PREFIX}{name}")
}

fn try_get_name(s: &str) -> Option<String> {
    s.strip_prefix(PARAMETER_PREFIX).map(decode_unicode_escapes)
}

pub fn new_float_message(name: &str, value: f32) -> OscMessage {
    let addr = name_to_address(name);
    let clamped_value = value.clamp(0.0f32, 1.0f32);
    let args = vec![OscType::Float(clamped_value)];
    OscMessage { addr, args }
}

pub fn new_note_on_message(name: &str, param_type: &usize, value: f32) -> OscMessage {
    let addr = name_to_address(name);
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
    let addr = name_to_address(name);
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
    let addr = name_to_address(name);
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
    true
}

pub struct Sender {
    tx: Option<mpsc::SyncSender<OscMessage>>,
}

impl Sender {
    pub fn new() -> Self {
        Self { tx: None }
    }

    fn is_running(&self) -> bool {
        self.tx.is_some()
    }

    pub fn init(&mut self, port: u16) -> io::Result<()> {
        if self.is_running() {
            return Ok(());
        }
        let (tx, rx) = mpsc::sync_channel::<_>(16);
        self.tx = Some(tx);

        let sock = UdpSocket::bind("127.0.0.1:0")?;
        sock.connect(("127.0.0.1", port))?;

        let mut state = HashMap::new();
        let mut sender_func = move |msg: OscMessage| {
            if let Some(prev) = state.get(&msg.addr) {
                if is_nearly_eq(prev, &msg) {
                    return None;
                }
            }
            let packet = OscPacket::Message(msg.clone());
            let buf = encoder::encode(&packet).ok()?;
            sock.send(&buf)
                .inspect_err(|e| nih_error!("failed to send: {e}"))
                .ok()?;
            state.insert(msg.addr.clone(), msg);
            Some(())
        };
        thread::spawn(move || {
            for msg in rx {
                sender_func(msg);
            }
        });
        Ok(())
    }

    pub fn send(&self, msg: OscMessage) -> bool {
        self.tx.as_ref().is_some_and(|tx| tx.try_send(msg).is_ok())
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

    pub fn is_running(&self) -> bool {
        self.join_handle.is_some()
    }

    pub fn init(&mut self, port: u16) -> io::Result<()> {
        if self.stop.load(Ordering::Acquire) {
            return Ok(());
        }

        let sock = UdpSocket::bind(("127.0.0.1", port))?;
        sock.set_read_timeout(Some(Duration::from_millis(100)))?;

        let state = self.state.clone();
        let stop = self.stop.clone();
        let mut buf = [0; decoder::MTU];
        let mut receiver_func = move |sock: &UdpSocket| {
            let (len, _) = sock.recv_from(&mut buf).ok()?;
            let (_, packet) = decoder::decode_udp(&buf[..len]).ok()?;
            if let OscPacket::Message(msg) = packet {
                Some((try_get_name(&msg.addr)?, msg.args.first()?.clone()))
            } else {
                None
            }
        };
        let join_handle = thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                if let Some((name, t)) = receiver_func(&sock) {
                    state.write().unwrap().insert(name, t);
                }
            }
        });
        self.join_handle = Some(join_handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let join_handle = self.join_handle.take();
        if let Some(join_handle) = join_handle {
            join_handle
                .join()
                .unwrap_or_else(|_e| nih_error!("failed to join to receiver thread"));
        }
        self.stop.store(false, Ordering::Release);
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop();
    }
}
