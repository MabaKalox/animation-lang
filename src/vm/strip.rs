use crate::color_intermeddle_type::ColorMiddleLayer;

pub struct DummyLedStrip {
    length: usize,
    buf: Vec<ColorMiddleLayer>,
}

impl DummyLedStrip {
    pub fn length(&self) -> u32 {
        self.length as u32
    }

    pub fn blit(&mut self) {}

    pub fn set_pixel(&mut self, idx: u32, color: ColorMiddleLayer) {
        self.buf[idx as usize] = color;
    }

    pub fn get_pixel(&self, idx: u32) -> ColorMiddleLayer {
        self.buf[idx as usize].clone()
    }

    pub fn export(&self) -> Box<dyn Iterator<Item = ColorMiddleLayer> + Send> {
        Box::new(self.buf.clone().into_iter())
    }
}

impl DummyLedStrip {
    pub fn new(length: usize) -> Self {
        Self {
            length,
            buf: vec![ColorMiddleLayer::new(0, 0, 0, 0); length],
        }
    }
}
