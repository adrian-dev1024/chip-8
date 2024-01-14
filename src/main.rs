mod chip8;
mod io;

use chip8::Chip8;
use io::IOContext;

pub fn main() -> Result<(), String> {
    // Screen setup (sdl2)
    let mut io_context = IOContext::new()?;

    // Initialize the Chip8 system and load the game into the memory
    let mut chip8 = Chip8::new(io_context.renderer);
    chip8.load_game("/Users/adriangaray/workspace/rust/chip-8/games/tetris.c8")?;

    'running: loop {
        chip8.emulate_cycle()?;

        io_context.keyboard.keys_pressed(&mut chip8.keys);

        if io_context.keyboard.quit {
            break 'running;
        }

        // for event in event_pump.poll_iter() {
        //     match event {
        //         Event::Quit { .. }
        //         | Event::KeyDown {
        //             keycode: Some(Keycode::Escape),
        //             ..
        //         } => break 'running,
        //         _ => {}
        //     }
        // }

        // io_context.renderer.clear();
        // io_context.renderer.present();
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        // The rest of the game loop goes here...
    }

    Ok(())
}
