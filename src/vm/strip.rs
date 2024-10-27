use crate::vm::RGBW8;
use smart_leds_trait::White;
use std::iter::repeat;

const RGBW_BLACK: RGBW8 = RGBW8::new_alpha(0, 0, 0, White(0));

pub struct DummyLedStrip {
    buf: Vec<RGBW8>,
}

impl DummyLedStrip {
    pub fn length(&self) -> u32 {
        self.buf.len() as u32
    }

    pub fn blit(&mut self) {}

    pub fn set_pixel(&mut self, idx: u32, color: RGBW8) {
        self.buf[idx as usize] = color;
    }

    pub fn get_pixel(&self, idx: u32) -> RGBW8 {
        self.buf[idx as usize]
    }

    pub fn export(&self) -> Box<dyn Iterator<Item = RGBW8> + Send> {
        Box::new(self.buf.clone().into_iter())
    }

    pub fn set_length(&mut self, length: usize) {
        if length < self.buf.len() {
            self.buf.truncate(length);
        } else {
            self.buf
                .extend(repeat(RGBW_BLACK).take(length - self.buf.len()));
        }
    }
}

impl DummyLedStrip {
    pub fn new(length: usize) -> Self {
        Self {
            buf: vec![RGBW_BLACK; length],
        }
    }
}
