use std::iter::repeat;

use rgb::RGB8;

const RGB8_BLACK: RGB8 = RGB8::new(0, 0, 0);

pub struct DummyLedStrip {
    buf: Vec<RGB8>,
}

impl DummyLedStrip {
    pub fn length(&self) -> u32 {
        self.buf.len() as u32
    }

    pub fn blit(&mut self) {}

    pub fn set_pixel(&mut self, idx: u32, color: RGB8) {
        self.buf[idx as usize] = color;
    }

    pub fn get_pixel(&self, idx: u32) -> RGB8 {
        self.buf[idx as usize]
    }

    pub fn export(&self) -> Box<dyn Iterator<Item = RGB8> + Send> {
        Box::new(self.buf.clone().into_iter())
    }

    pub fn set_length(&mut self, length: usize) {
        if length < self.buf.len() {
            self.buf.truncate(length);
        } else {
            self.buf
                .extend(repeat(RGB8_BLACK).take(length - self.buf.len()));
        }
    }
}

impl DummyLedStrip {
    pub fn new(length: usize) -> Self {
        Self {
            buf: vec![RGB8_BLACK; length],
        }
    }
}
