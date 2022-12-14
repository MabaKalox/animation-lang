use embedded_graphics::pixelcolor::Rgb888;
use smart_leds_trait::{White, RGBW};

pub type RGBW8 = RGBW<u8, u8>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorMiddleLayer(pub RGBW8);

impl ColorMiddleLayer {
    pub fn set_alpha(&mut self, alpha: u8) {
        self.0.a = White(alpha);
    }
}

impl ColorMiddleLayer {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        ColorMiddleLayer(RGBW8::new_alpha(r, g, b, White(a)))
    }
}

impl From<ColorMiddleLayer> for RGBW8 {
    fn from(value: ColorMiddleLayer) -> RGBW8 {
        value.0
    }
}

impl From<RGBW8> for ColorMiddleLayer {
    fn from(value: RGBW8) -> Self {
        ColorMiddleLayer(value)
    }
}

impl From<ColorMiddleLayer> for Rgb888 {
    fn from(value: ColorMiddleLayer) -> Rgb888 {
        Rgb888::new(value.0.r, value.0.g, value.0.b)
    }
}
