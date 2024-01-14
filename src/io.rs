use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::video::Window;
use sdl2::{EventPump, Sdl};

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

        self.canvas.present();

        Ok(())
    }

    pub fn draw_dot(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.draw_rect(x, y, 1, 1, DRAWING_COLOR)?;
        Ok(())
    }

    pub fn clear_dot(&mut self, x: usize, y: usize) -> Result<(), String> {
        self.draw_rect(x, y, 1, 1, BACKGROUND_COLOR)?;
        Ok(())
    }
}

// #[derive(Copy, Clone)]
enum Keys {
    Num1 = 0,
    Num2 = 1,
    Num3 = 2,
    Num4 = 3,
    Q = 4,
    W = 5,
    E = 6,
    R = 7,
    A = 8,
    S = 9,
    D = 10,
    F = 11,
    Z = 12,
    X = 13,
    C = 14,
    V = 15,
}

pub struct Keyboard {
    event_pump: EventPump,
    pub quit: bool,
}

impl Keyboard {
    pub fn new(sdl_context: Sdl) -> Keyboard {
        let event_pump = sdl_context.event_pump().unwrap();
        Keyboard {
            event_pump,
            quit: false,
        }
    }

    pub fn keys_pressed(&mut self, keys: &mut [u16; 16]) {
        keys.fill(0);
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.quit = true,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Num1 => keys[Keys::Num1 as usize] = 1,
                    Keycode::Num2 => keys[Keys::Num2 as usize] = 1,
                    Keycode::Num3 => keys[Keys::Num3 as usize] = 1,
                    Keycode::Num4 => keys[Keys::Num4 as usize] = 1,

                    Keycode::Q => keys[Keys::Q as usize] = 1,
                    Keycode::W => keys[Keys::W as usize] = 1,
                    Keycode::E => keys[Keys::E as usize] = 1,
                    Keycode::R => keys[Keys::R as usize] = 1,

                    Keycode::A => keys[Keys::A as usize] = 1,
                    Keycode::S => keys[Keys::S as usize] = 1,
                    Keycode::D => keys[Keys::D as usize] = 1,
                    Keycode::F => keys[Keys::F as usize] = 1,

                    Keycode::Z => keys[Keys::Z as usize] = 1,
                    Keycode::X => keys[Keys::X as usize] = 1,
                    Keycode::C => keys[Keys::C as usize] = 1,
                    Keycode::V => keys[Keys::V as usize] = 1,
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
