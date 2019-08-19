use std::io::{stdout, Stdout, Write};
use termion::color::{Fg, Rgb};
use termion::style::Reset;

pub const GRAY: Fg<Rgb> = Fg(Rgb(153, 153, 153));
pub const GREEN: Fg<Rgb> = Fg(Rgb(62, 75, 14));
pub const RED: Fg<Rgb> = Fg(Rgb(100, 42, 39));

pub struct Logger {
    writer: Stdout,
}

#[allow(dead_code)]
impl Logger {
    pub fn new() -> Logger {
        Logger { writer: stdout() }
    }

    pub fn info<S: Into<String>>(&mut self, message: S) {
        let message = format!("{}INFO{}: {}{}\n", GREEN, GRAY, Reset, message.into());

        self.writer
            .write(message.as_bytes())
            .expect("Failed to write to stdout");
    }

    pub fn error<S: Into<String>>(&mut self, message: S) {
        let message = format!("{}ERROR{}: {}{}\n", RED, GRAY, Reset, message.into());

        self.writer
            .write(message.as_bytes())
            .expect("Failed to write to stdout");
    }
}
