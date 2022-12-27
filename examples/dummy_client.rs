use std::time::{Duration, Instant};

use animation_lang::compiler::FromSource;
use animation_lang::vm::VMState;
use animation_lang::{
    program::Program,
    vm::{VMStateConfig, VM},
};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{IntoStorage, Point, RgbColor, Size};
use embedded_graphics::primitives::{Primitive, PrimitiveStyleBuilder, Rectangle, StrokeAlignment};
use minifb::{Key, Window, WindowOptions};
use rgb::RGB8;
use smart_leds_trait::SmartLedsWrite;
use tiny_http::{Method, Response, Server, StatusCode};

const VLED_QUANTITY: usize = 50;
const VLED_WIDTH: usize = 15;
const VLED_HEIGHT: usize = 15;
const VLED_H_SPACING: usize = 3;
const VLED_BORDER_WIDTH: usize = 0;

const WINDOW_PADDING: usize = 5;
const WINDOW_BG: Rgb888 = Rgb888::BLACK;
const WIDTH: usize =
    VLED_QUANTITY * (VLED_WIDTH + VLED_H_SPACING) - VLED_H_SPACING + 2 * WINDOW_PADDING;
const HEIGHT: usize = VLED_HEIGHT + 2 * WINDOW_PADDING;
const FB_SIZE: usize = WIDTH * HEIGHT;
//                                                                   / <- FPS here
const FRAME_TIME: Duration = Duration::from_micros(((1_f32 / 32_f32) * 1_000_000_f32) as u64);
const MAIN_LOOP_TIME: Duration = Duration::from_millis(1);
const DEFAULT_PROG: &str = include_str!("../../animation_lang/example_progs/blink.txt");

trait ToIndex<T> {
    fn to_index(&self) -> usize;
}

impl ToIndex<Point> for Point {
    fn to_index(&self) -> usize {
        self.x as usize + self.y as usize * WIDTH
    }
}

fn main() {
    let mut frame_buffer = [WINDOW_BG.into_storage(); FB_SIZE];
    // let example_bytecode = vec![];

    let mut window = Window::new(
        "dummy_led_client",
        WIDTH,
        HEIGHT,
        WindowOptions {
            borderless: true,
            ..Default::default()
        },
    )
    .unwrap();

    let mut server = Server::http("127.0.0.1:8888").unwrap();

    let mut led_strip = VLedStrip::new(VLED_QUANTITY);
    let mut vm_state = VM::new(VLED_QUANTITY, Default::default()).start(
        Program::from_source(DEFAULT_PROG).unwrap(),
        VMStateConfig {
            local_instruction_limit: Some(1_000_000),
            ..Default::default()
        },
    );
    let mut vm_running = true;

    let mut last_update = Instant::now() - 2 * FRAME_TIME;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if let Some(new_prog) = try_receive_new_prob(&mut server) {
            vm_state = {
                let (vm, state_config, _) = vm_state.stop();
                vm.start(Program::from_binary(new_prog), state_config)
            };
            vm_running = true;
        }

        let now = Instant::now();
        if vm_running && now > last_update + FRAME_TIME {
            match vm_state.next() {
                Some(r) => match r {
                    Ok(frame) => led_strip.write(frame).unwrap(),
                    Err(e) => {
                        println!("halting vm until new program received");
                        eprintln!("{:?}", e);
                        vm_running = false;
                    }
                },
                None => {
                    println!("program ended - restarting");
                    vm_state = vm_state.reset(None);
                }
            }
            led_strip.export_fb(&mut frame_buffer);

            window
                .update_with_buffer(&frame_buffer, WIDTH, HEIGHT)
                .unwrap();

            last_update = now;
        } else {
            window.update();
        }

        let passed = last_update.elapsed();
        if passed < MAIN_LOOP_TIME {
            std::thread::sleep(MAIN_LOOP_TIME - passed);
        }
    }
}

trait Restart {
    fn reset(self, new_program: Option<Program>) -> Self;
}

impl Restart for VMState {
    fn reset(self, new_program: Option<Program>) -> Self {
        let (vm, config, old_program) = self.stop();
        if let Some(program) = new_program {
            vm.start(program, config)
        } else {
            vm.start(old_program, config)
        }
    }
}

fn try_receive_new_prob(server: &mut Server) -> Option<Vec<u8>> {
    match server.try_recv().unwrap() {
        Some(mut req) => match req.url() {
            "/send_prog_base64" => match req.method() {
                Method::Post => {
                    let mut body = Vec::new();
                    req.as_reader().read_to_end(&mut body).unwrap();

                    match base64::decode(&body) {
                        Ok(prog) => {
                            req.respond(Response::empty(200)).unwrap();
                            Some(prog)
                        }
                        Err(e) => {
                            let message = format!("bad base64 payload: {}", e);
                            req.respond(Response::new(
                                StatusCode(400),
                                vec![],
                                message.as_bytes(),
                                Some(message.as_bytes().len()),
                                None,
                            ))
                            .unwrap();
                            None
                        }
                    }
                }
                _ => {
                    req.respond(Response::empty(StatusCode(405))).unwrap();
                    None
                }
            },
            _ => {
                req.respond(Response::empty(StatusCode(404))).unwrap();
                None
            }
        },
        None => None,
    }
}

trait ExportFB {
    fn export_fb(&self, frame_buffer: &mut [u32]);
}

impl ExportFB for VLedStrip {
    fn export_fb(&self, frame_buffer: &mut [u32]) {
        for (i, color_rgbw) in self.state.iter().enumerate() {
            let vled_color = Rgb888::new(color_rgbw.r, color_rgbw.g, color_rgbw.b);
            let horizontal_offset = i * (VLED_WIDTH + VLED_H_SPACING);
            let virtual_led = Rectangle::new(
                Point::new(
                    (horizontal_offset + WINDOW_PADDING) as i32,
                    WINDOW_PADDING as i32,
                ),
                Size::new(VLED_WIDTH as u32, VLED_HEIGHT as u32),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(vled_color)
                    .stroke_width(VLED_BORDER_WIDTH as u32)
                    .stroke_alignment(StrokeAlignment::Inside)
                    .stroke_color(Rgb888::WHITE)
                    .build(),
            );

            for p in virtual_led.pixels() {
                frame_buffer[p.0.to_index()] = p.1.into_storage();
            }
        }
    }
}

pub struct VLedStrip {
    pub state: Vec<RGB8>,
}

impl SmartLedsWrite for VLedStrip {
    type Error = ();
    type Color = RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>,
    {
        for (i, v) in iterator.take(self.state.len()).enumerate() {
            self.state[i] = v.into();
        }
        Ok(())
    }
}

impl VLedStrip {
    pub fn new(length: usize) -> Self {
        VLedStrip {
            state: vec![RGB8::new(0, 0, 0); length],
        }
    }
}
