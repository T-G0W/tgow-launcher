use std::io::{BufRead, BufReader, Read, Write};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream};

const SOCKET: &str = "tgow-launcher.sock";
const LIMIT: u64 = 1024;

pub(crate) static WAKER: OnceLock<egui::Context> = OnceLock::new();

pub(crate) enum Claim {
    First(Receiver<String>),
    Forwarded,
    Unavailable,
}

pub(crate) fn claim(message: &str) -> Claim {
    let Ok(name) = SOCKET.to_ns_name::<GenericNamespaced>() else {
        return Claim::Unavailable;
    };
    if let Ok(mut stream) = Stream::connect(name.clone()) {
        let line = message.replace(['\n', '\r'], " ");
        return if writeln!(stream, "{line}").is_ok() {
            Claim::Forwarded
        } else {
            Claim::Unavailable
        };
    }
    let Ok(listener) = ListenerOptions::new().name(name).create_sync() else {
        return Claim::Unavailable;
    };
    let (sender, receiver) = mpsc::channel();
    let _ = thread::Builder::new()
        .name("instance".to_owned())
        .spawn(move || {
            for connection in listener.incoming().flatten() {
                let mut line = String::new();
                let mut reader = BufReader::new(connection).take(LIMIT);
                if reader.read_line(&mut line).is_ok() {
                    let _ = sender.send(line.trim().to_owned());
                    if let Some(ctx) = WAKER.get() {
                        ctx.request_repaint();
                    }
                }
            }
        });
    Claim::First(receiver)
}
