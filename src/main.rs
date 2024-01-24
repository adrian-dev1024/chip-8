mod chip8;
mod io;

use chip8::Chip8;
use io::IOContext;

pub fn main() -> Result<(), String> {
    let path_str = std::env::args().nth(1).expect("no path given");
    let path = std::path::PathBuf::from(path_str);

    // Screen setup (sdl2)
    let mut io_context = IOContext::new()?;

    // Initialize the Chip8 system and load the game into the memory
    let mut chip8 = Chip8::new();
    chip8.load_game(path);

    chip8.run_loop(&mut io_context)?;

    Ok(())
}
