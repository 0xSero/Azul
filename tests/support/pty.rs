#![allow(dead_code)]

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

pub enum Key {
    Tab,
    Enter,
    Esc,
    Up,
    Down,
    Left,
    Right,
    CtrlC,
}

pub struct PtyHarness {
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    parser: vt100::Parser,
    child: Box<dyn portable_pty::Child + Send>,
}

impl PtyHarness {
    pub fn spawn(bin: &str, size: PtySize, env: &[(String, String)]) -> Self {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(size).expect("failed to open pty");

        let mut cmd = CommandBuilder::new(bin);
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd).expect("failed to spawn app");
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("failed to clone reader");
        let writer = pair.master.take_writer().expect("failed to take writer");

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let parser = vt100::Parser::new(size.rows, size.cols, 0);

        Self {
            writer,
            rx,
            parser,
            child,
        }
    }

    pub fn send(&mut self, input: &str) {
        self.send_bytes(input.as_bytes());
    }

    pub fn send_key(&mut self, key: Key) {
        let seq = match key {
            Key::Tab => "\t",
            Key::Enter => "\r",
            Key::Esc => "\u{1b}",
            Key::Up => "\u{1b}[A",
            Key::Down => "\u{1b}[B",
            Key::Right => "\u{1b}[C",
            Key::Left => "\u{1b}[D",
            Key::CtrlC => "\u{3}",
        };
        self.send(seq);
    }

    pub fn send_bytes(&mut self, bytes: &[u8]) {
        self.writer
            .write_all(bytes)
            .expect("failed to write to pty");
        self.writer.flush().expect("failed to flush pty");
    }

    pub fn drain_for(&mut self, duration: Duration) {
        let start = Instant::now();
        while start.elapsed() < duration {
            match self.rx.recv_timeout(Duration::from_millis(20)) {
                Ok(data) => self.parser.process(&data),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    pub fn wait_for(&mut self, needle: &str, timeout: Duration) -> String {
        let start = Instant::now();
        loop {
            self.drain_for(Duration::from_millis(60));
            let contents = self.contents();
            if contents.contains(needle) {
                return contents;
            }
            if start.elapsed() > timeout {
                panic!("timed out waiting for {}", needle);
            }
        }
    }

    pub fn wait_for_absence(&mut self, needle: &str, timeout: Duration) {
        let start = Instant::now();
        loop {
            self.drain_for(Duration::from_millis(60));
            let contents = self.contents();
            if !contents.contains(needle) {
                return;
            }
            if start.elapsed() > timeout {
                panic!("timed out waiting for absence of {}", needle);
            }
        }
    }

    pub fn contents(&self) -> String {
        self.parser.screen().contents()
    }

    pub fn wait_for_exit(mut self, timeout: Duration) {
        let start = Instant::now();
        loop {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            if start.elapsed() > timeout {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
