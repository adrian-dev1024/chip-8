use std::fs;

use crate::io::Renderer;
use rand::Rng;

const CHIP8_FONT_SET: [u16; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Chip8 {
    // current opcode
    op_code: u16,
    // 4K memory
    memory: [u16; 4096],
    // general purpose registers named v0 - vf
    v: [u16; 16],
    // Index register
    i: usize,
    // value from 0x000 to 0xFFF
    program_counter: usize,
    // screen with 2048 pixels (64 x 32)
    gfx: [u8; 64 * 32],
    delay_timer: u16,
    sound_timer: u16,
    stack: [u16; 16],
    stack_pointer: usize,
    // keypad current state
    pub keys: [u16; 16],
    renderer: Renderer,
}

impl Chip8 {
    pub fn new(mut renderer: Renderer) -> Chip8 {
        // Clear display
        renderer.clear();

        // Clear memory
        let mut memory = [0; 4096];

        // Load font set
        for number in 0..80 {
            memory[number] = CHIP8_FONT_SET[number];
        }

        Chip8 {
            op_code: 0,
            memory,
            v: [0; 16],
            i: 0,
            program_counter: 0x200,
            gfx: [0; 64 * 32],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keys: [0; 16],
            renderer,
        }
    }

    pub fn emulate_cycle(&mut self) -> Result<(), String> {
        // TODO: Consider making op_code a local var

        // Fetch Opcode
        self.op_code = self.fetch_op_code();

        println!("Program Counter: {}", self.program_counter);
        println!("Opcode: {:#06x}", self.op_code);

        // Decode Opcode
        match self.op_code & 0xF000 {
            0x0000 => match self.op_code & 0x000F {
                // 0x00E0: Clears the screen
                0x0000 => self.clear_screen(),
                // 0x00EE: Returns from subroutine
                0x000E => {
                    self.stack_pointer -= 1;
                    self.program_counter =
                        usize::try_from(self.stack[self.stack_pointer] & 0x0FFF).unwrap() + 2;
                }
                _ => panic!("Unknown opcode [0x0000]: {:#06x}", self.op_code),
            },
            // 0x1NNN: goto NNN
            0x1000 => self.program_counter = usize::try_from(self.op_code & 0x0FFF).unwrap(), // 0x1NNN goto NNN
            // 2NNN: Calls subroutine at NNN
            0x2000 => {
                self.stack[self.stack_pointer] = u16::try_from(self.program_counter).unwrap();
                self.stack_pointer += 1;
                self.program_counter = usize::try_from(self.op_code & 0x0FFF).unwrap();
            }
            // 3XNN: Skips the next instruction if VX equals NN
            0x3000 => {
                let x = (usize::try_from(self.op_code).unwrap() & 0x0F00) >> 8;
                let val = self.op_code & 0x00FF;
                if self.v[x] == val {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            // 4XNN: Skips the next instruction if VX does not equal NN
            0x4000 => {
                let x = (usize::try_from(self.op_code).unwrap() & 0x0F00) >> 8;
                let val = self.op_code & 0x00FF;
                if self.v[x] != val {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            // 5XY0: Skips the next instruction if VX equals VY
            0x5000 => {
                let x = (usize::try_from(self.op_code).unwrap() & 0x0F00) >> 8;
                let y = (usize::try_from(self.op_code).unwrap() & 0x00F0) >> 4;
                if self.v[x] == self.v[y] {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            // 6XNN: Sets VX to NN
            0x6000 => {
                let x = (usize::try_from(self.op_code).unwrap() & 0x0F00) >> 8;
                let val = self.op_code & 0x00FF;
                self.v[x] = val;
                self.program_counter += 2;
            }
            // 7XNN: Adds NN to VX
            0x7000 => {
                let x = (usize::try_from(self.op_code).unwrap() & 0x0F00) >> 8;
                let val = self.op_code & 0x00FF;
                self.v[x] += val;
                self.program_counter += 2;
            }
            0x8000 => match self.op_code & 0x000F {
                // 8XY0: Sets VX to the value of VY
                0x0000 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    self.v[x] = self.v[y];
                    self.program_counter += 2;
                }
                // 8XY1: Sets VX to VX or VY
                0x0001 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    self.v[x] |= self.v[y];
                    self.program_counter += 2;
                }
                // 8XY2: Sets VX to VX and VY
                0x0002 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    self.v[x] &= self.v[y];
                    self.program_counter += 2;
                }
                // 8XY3: Sets VX to VX xor VY
                0x0003 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    self.v[x] ^= self.v[y];
                    self.program_counter += 2;
                }
                // 8XY4: Adds VY to VX (overflow)
                0x0004 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    if self.v[y] > self.v[x] {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[x] += self.v[y];
                    self.program_counter += 2;
                }
                // 8XY5: VY is subtracted from VX (overflow)
                0x0005 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    if self.v[x] >= self.v[y] {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[x] -= self.v[y];
                    self.program_counter += 2;
                }
                // 8XY6: Stores the least significant bit of VX in VF and then shifts VX to the right by 1
                0x0006 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.v[15] = self.v[x] & 0x000F;
                    self.v[x] >>= 1;
                    self.program_counter += 2;
                }
                // 8XY7: Sets VX to VY minus VX (overflow)
                0x0007 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let y = (op_code & 0x00F0) >> 4;
                    if self.v[y] >= self.v[x] {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }
                    self.v[x] = self.v[y] - self.v[x];
                    self.program_counter += 2;
                }
                // 8XYE: Stores the most significant bit of VX in VF and then shifts VX to the left by 1
                0x000E => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.v[15] = self.v[x] & 0xF000 >> 12;
                    self.v[x] <<= 1;
                    self.program_counter += 2;
                }
                _ => panic!("Unknown opcode [0x8000]: {:#06x}", self.op_code),
            },
            // 9XY0: Skips the next instruction if VX does not equal VY
            0x9000 => {
                let op_code = usize::try_from(self.op_code).unwrap();
                let x = (op_code & 0x0F00) >> 8;
                let y = (op_code & 0x00F0) >> 4;
                if self.v[x] != self.v[y] {
                    self.program_counter += 4;
                } else {
                    self.program_counter += 2;
                }
            }
            // ANNN: Sets I to the address NNN
            0xA000 => {
                self.i = usize::try_from(self.op_code & 0x0FFF).unwrap();
                self.program_counter += 2;
            }
            // BNNN: Jumps to the address NNN plus V0
            0xB000 => {
                self.program_counter = usize::try_from(self.v[0] + (self.op_code & 0x0FFF)).unwrap()
            }
            // CXNN: Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN
            0xC000 => {
                let op_code = usize::try_from(self.op_code).unwrap();
                let x = (op_code & 0x0F00) >> 8;
                let val = self.op_code & 0x00FF;
                self.v[x] = rand::thread_rng().gen_range(0..255) & val;
                self.program_counter += 2;
            }
            // DXYN: Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels.
            0xD000 => {
                self.draw()?;
                self.program_counter += 2;
            }
            0xE000 => match self.op_code & 0x000F {
                // EX9E: Skips the next instruction if the key stored in VX is pressed
                0x000E => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let index = usize::try_from(self.v[x]).unwrap();
                    if self.keys[index] != 0 {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                // EXA1: Skips the next instruction if the key stored in VX is not pressed
                0x0001 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let index = usize::try_from(self.v[x]).unwrap();
                    if self.keys[index] == 0 {
                        self.program_counter += 4;
                    } else {
                        self.program_counter += 2;
                    }
                }
                _ => panic!("Unknown opcode [0xE000]: {:#06x}", self.op_code),
            },
            0xF000 => match self.op_code & 0x00FF {
                // FX07: Sets VX to the value of the delay timer
                0x0007 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.v[x] = self.delay_timer;
                    self.program_counter += 2;
                }
                // FX0A: A key press is awaited, and then stored in VX (blocking operation, all instruction halted until next key event)
                0x000A => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    let mut key_pressed = false;
                    for i in 0..16 {
                        if self.keys[i] == 1 {
                            key_pressed = true;
                            self.v[x] = u16::try_from(i).unwrap();
                            break;
                        }
                    }

                    if !key_pressed {
                        return Ok(());
                    }

                    self.program_counter += 2;
                }
                // FX15: Sets the delay timer to VX
                0x0015 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.delay_timer = self.v[x];
                    self.program_counter += 2;
                }
                // FX18: Sets the sound timer to VX
                0x0018 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.sound_timer = self.v[x];
                    self.program_counter += 2;
                }
                // FX1E: Adds VX to I. VF is not affected
                0x001E => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.i += usize::try_from(self.v[x]).unwrap();
                    self.program_counter += 2;
                }
                // FX29: Sets I to the location of the sprite for the character in VX.
                0x0029 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    self.i = usize::try_from(self.v[x]).unwrap();
                    self.program_counter += 2;
                }
                // FX33: Stores the binary-coded decimal representation of VX, with the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2
                0x0033 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    self.memory[self.i] = self.v[(op_code & 0x0F00) >> 8] / 100;
                    self.memory[self.i + 1] = (self.v[(op_code & 0x0F00) >> 8] / 10) % 10;
                    self.memory[self.i + 2] = (self.v[(op_code & 0x0F00) >> 8] % 100) % 10;
                    self.program_counter += 2;
                }
                // FX55: Stores from V0 to VX (including VX) in memory, starting at address I. The offset from I is increased by 1 for each value written, but I itself is left unmodified.
                0x0055 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    for n in 0..(x + 1) {
                        self.memory[self.i + n] = self.v[n]
                    }
                    self.program_counter += 2;
                }
                // FX65: Fills from V0 to VX (including VX) with values from memory, starting at address I. The offset from I is increased by 1 for each value read, but I itself is left unmodified
                0x0065 => {
                    let op_code = usize::try_from(self.op_code).unwrap();
                    let x = (op_code & 0x0F00) >> 8;
                    for n in 0..(x + 1) {
                        self.v[n] = self.memory[self.i + n]
                    }
                    self.program_counter += 2;
                }
                _ => panic!("Unknown opcode [0xF000]: {:#06x}", self.op_code),
            },

            // More opcodes //
            _ => panic!("Unknown opcode: {}", self.op_code),
        }

        // Update timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP!")
            }
            self.sound_timer -= 1;
        }
        Ok(())
    }

    fn fetch_op_code(&mut self) -> u16 {
        self.memory[self.program_counter] << 8 | self.memory[self.program_counter + 1]
    }

    fn clear_screen(&mut self) {
        self.gfx.fill(0);
        self.renderer.clear();
    }

    fn draw(&mut self) -> Result<(), String> {
        let op_code = usize::try_from(self.op_code).unwrap();
        let x = self.v[(op_code & 0x0F00) >> 8];
        let y = self.v[(op_code & 0x00F0) >> 4];
        let height = op_code & 0x000F;
        self.v[15] = 0;

        for y_line in 0..height {
            let pixel = self.memory[self.i + y_line];
            for x_line in 0..8 {
                if (pixel & (0x80 >> x_line)) != 0 {
                    let x_size = usize::try_from(x).unwrap();
                    let y_size = usize::try_from(y).unwrap();
                    if self.gfx[x_size + x_line + ((y_size + y_line) * 64)] == 1 {
                        self.v[15] = 1;
                    }
                    self.gfx[x_size + x_line + ((y_size + y_line) * 64)] ^= 1;

                    if self.gfx[x_size + x_line + ((y_size + y_line) * 64)] == 1 {
                        self.renderer.draw_dot(x_size + x_line, y_size + y_line)?;
                    } else {
                        self.renderer.clear_dot(x_size + x_line, y_size + y_line)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn load_game(&mut self, game_file_path: &str) -> Result<(), String> {
        let contents = fs::read(game_file_path).unwrap();
        let mut index = self.program_counter;
        for val in contents {
            self.memory[index] = u16::try_from(val).unwrap();
            index += 1;
        }

        Ok(())
    }
}
