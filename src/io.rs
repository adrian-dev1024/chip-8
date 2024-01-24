use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::video::Window;
use sdl2::{EventPump, Sdl};

use crate::chip8::ChipState;

const DISPLAY_WIDTH: u16 = 64;
const DISPLAY_HEIGHT: u16 = 32;
const SCALE: u16 = 10;
const BACKGROUND_COLOR: Color = Color::BLACK;
const DRAWING_COLOR: Color = Color::WHITE;

pub struct IOContext {
    pub renderer: Renderer,
    pub keyboard: Keyboard,
}

impl IOContext {
    pub fn new() -> Result<IOContext, String> {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window(
                "Chip-8",
                (DISPLAY_WIDTH * SCALE).try_into().unwrap(),
                (DISPLAY_HEIGHT * SCALE).try_into().unwrap(),
            )
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let renderer = Renderer::new(window)?;

        let keyboard = Keyboard::new(sdl_context);

        Ok(IOContext { renderer, keyboard })
    }
}

pub struct Renderer {
    canvas: WindowCanvas,
}

impl Renderer {
    pub fn new(window: Window) -> Result<Renderer, String> {
        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

        Ok(Renderer { canvas })
    }

    pub fn draw(&mut self, gfx: [u8; 64 * 32]) -> Result<(), String> {
        self.clear();
        for (i, pix) in gfx.iter().enumerate() {
            if *pix == 1 {
                let x = i % 64;
                let y = i / 64;
                self.draw_dot(x, y)?;
            }
        }
        self.canvas.present();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.canvas.set_draw_color(BACKGROUND_COLOR);
        self.canvas.clear();
    }

    fn draw_rect(
        &mut self,
        x: usize,
        y: usize,
        width: u16,
        height: u16,
        color: Color,
    ) -> Result<(), String> {
        let x = u16::try_from(x).unwrap();
        let y = u16::try_from(y).unwrap();
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(Rect::new(
            (x * SCALE).try_into().unwrap(),
            (y * SCALE).try_into().unwrap(),
            (width * SCALE).try_into().unwrap(),
            (height * SCALE).try_into().unwrap(),
        ))?;

        // self.canvas.present();

        Ok(())
    }

    fn draw_dot(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.draw_rect(x, y, 1, 1, DRAWING_COLOR)?;
        Ok(())
    }
}

pub struct Keyboard {
    event_pump: EventPump,
}

impl Keyboard {
    pub fn new(sdl_context: Sdl) -> Keyboard {
        let event_pump = sdl_context.event_pump().unwrap();
        Keyboard { event_pump }
    }

    pub fn keys_pressed(&mut self, keys: &mut [u8; 16], state: &mut ChipState) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => *state = ChipState::Quit,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Num1 => keys[0] = 1,
                    Keycode::Num2 => keys[1] = 1,
                    Keycode::Num3 => keys[2] = 1,
                    Keycode::Num4 => keys[3] = 1,

                    Keycode::Q => keys[4] = 1,
                    Keycode::W => keys[5] = 1,
                    Keycode::E => keys[6] = 1,
                    Keycode::R => keys[7] = 1,

                    Keycode::A => keys[8] = 1,
                    Keycode::S => keys[9] = 1,
                    Keycode::D => keys[10] = 1,
                    Keycode::F => keys[11] = 1,

                    Keycode::Z => keys[12] = 1,
                    Keycode::X => keys[13] = 1,
                    Keycode::C => keys[14] = 1,
                    Keycode::V => keys[15] = 1,
                    Keycode::Space => {
                        *state = if *state == ChipState::Pause {
                            ChipState::Run
                        } else {
                            ChipState::Pause
                        }
                    }
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Num1 => keys[0] = 0,
                    Keycode::Num2 => keys[1] = 0,
                    Keycode::Num3 => keys[2] = 0,
                    Keycode::Num4 => keys[3] = 0,

                    Keycode::Q => keys[4] = 0,
                    Keycode::W => keys[5] = 0,
                    Keycode::E => keys[6] = 0,
                    Keycode::R => keys[7] = 0,

                    Keycode::A => keys[8] = 0,
                    Keycode::S => keys[9] = 0,
                    Keycode::D => keys[10] = 0,
                    Keycode::F => keys[11] = 0,

                    Keycode::Z => keys[12] = 0,
                    Keycode::X => keys[13] = 0,
                    Keycode::C => keys[14] = 0,
                    Keycode::V => keys[15] = 0,
                    _ => {}
                },
                _ => {}
            }
        }
        // println!("keys: {:?}", keys);
    }
}
