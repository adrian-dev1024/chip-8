use std::fs;
use std::path::PathBuf;

use rand::Rng;

use crate::io::IOContext;

const CHIP8_FONT_SET: [u8; 80] = [
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

#[derive(Debug, PartialEq)]
pub enum ChipState {
    Block,
    Run,
    Draw,
    Clear,
    Pause,
    Quit,
}

#[derive(Debug)]
pub struct Chip8 {
    // 4K memory
    memory: [u8; 4096],
    // general purpose registers named v0 - vf
    v: [u8; 16],
    // Index register
    i: usize,
    // value from 0x000 to 0xFFF
    program_counter: usize,
    // screen with 2048 pixels (64 x 32)
    pub gfx: [u8; 64 * 32],
    pub state: ChipState,
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; 16],
    stack_pointer: usize,
    // keypad current state
    pub keys: [u8; 16],
}

impl Chip8 {
    pub fn new() -> Chip8 {
        // Clear memory
        let mut memory = [0; 4096];

        // Load font set
        for number in 0..80 {
            memory[number] = CHIP8_FONT_SET[number];
        }

        Chip8 {
            memory,
            v: [0; 16],
            i: 0,
            program_counter: 0x200,
            gfx: [0; 64 * 32],
            state: ChipState::Run,
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            stack_pointer: 0,
            keys: [0; 16],
        }
    }

    pub fn load_game(&mut self, game_file_path: PathBuf) {
        let contents = fs::read(game_file_path).unwrap();
        let mut index = self.program_counter;
        for val in contents {
            self.memory[index] = u8::try_from(val).unwrap();
            index += 1;
        }
    }

    pub fn run_loop(&mut self, io_context: &mut IOContext) -> Result<(), String> {
        'running: loop {
            if self.state != ChipState::Pause {
                self.emulate_cycle();
            }

            io_context
                .keyboard
                .keys_pressed(&mut self.keys, &mut self.state);

            match self.state {
                ChipState::Draw => io_context.renderer.draw(self.gfx)?,
                ChipState::Clear => io_context.renderer.clear(),
                ChipState::Quit => break 'running,
                _ => {}
            }
        }

        Ok(())
    }

    pub fn emulate_cycle(&mut self) {
        if self.state != ChipState::Block {
            self.state = ChipState::Run;
        }

        self.execute();

        if self.state == ChipState::Block {
            return;
        }

        // Update timers
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
            ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
        }
        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                println!("BEEP!")
            }
            self.sound_timer -= 1;
        }
    }

    fn get_op_code(&self) -> u16 {
        u16::try_from(self.memory[self.program_counter]).unwrap() << 8
            | u16::try_from(self.memory[self.program_counter + 1]).unwrap()
    }

    fn execute(&mut self) {
        // Fetch Opcode
        let op_code = self.get_op_code();

        // Decode Opcode
        match op_code & 0xF000 {
            0x0000 => match op_code & 0x000F {
                // 0x00E0
                0x0000 => self.clear_screen(),
                // 0x00EE
                0x000E => self.return_from_subroutine(),
                _ => panic!("Unknown opcode [0x0000]: {:#06x}", op_code),
            },
            // 0x1NNN: goto NNN
            0x1000 => self.goto(op_code),
            // 2NNN
            0x2000 => self.call_subroutine(op_code),
            // 3XNN
            0x3000 => self.skip_if_eq_to_nn(op_code),
            // 4XNN
            0x4000 => self.skip_if_not_eq_to_nn(op_code),
            // 5XY0
            0x5000 => self.skip_if_vx_eq_to_vy(op_code),
            // 6XNN
            0x6000 => self.set_vx_to_nn(op_code),
            // 7XNN
            0x7000 => self.add_nn_to_vx(op_code),

            0x8000 => match op_code & 0x000F {
                // 8XY0
                0x0000 => self.set_vx_to_vy(op_code),
                // 8XY1
                0x0001 => self.set_vx_to_vx_or_vy(op_code),
                // 8XY2
                0x0002 => self.set_vx_to_vx_and_vy(op_code),
                // 8XY3
                0x0003 => self.set_vx_to_vx_xor_vy(op_code),
                // 8XY4
                0x0004 => self.set_vx_to_vx_plus_vy(op_code),
                // 8XY5
                0x0005 => self.set_vx_to_vx_minus_vy(op_code),
                // 8XY6
                0x0006 => self.shift_right(op_code),
                // 8XY7
                0x0007 => self.set_vx_to_vy_minus_vx(op_code),
                // 8XYE
                0x000E => self.shift_left(op_code),
                _ => panic!("Unknown opcode [0x8000]: {:#06x}", op_code),
            },
            // 9XY0
            0x9000 => self.skip_if_vx_not_eq_vy(op_code),
            // ANNN
            0xA000 => self.set_i_to_nnn(op_code),
            // BNNN
            0xB000 => self.goto_nnn_plus_v0(op_code),
            // CXNN
            0xC000 => self.set_vx_to_rand_and_nn(op_code),
            // DXYN
            0xD000 => self.draw(op_code),

            0xE000 => match op_code & 0x000F {
                // EX9E
                0x000E => self.skip_if_key_pressed(op_code),
                // EXA1
                0x0001 => self.skip_if_not_key_pressed(op_code),
                _ => panic!("Unknown opcode [0xE000]: {:#06x}", op_code),
            },

            0xF000 => match op_code & 0x00FF {
                // FX07
                0x0007 => self.set_vx_to_delay_timer(op_code),
                // FX0A
                0x000A => self.is_key_press(op_code),
                // FX15
                0x0015 => self.set_delay_timer_to_vx(op_code),
                // FX18
                0x0018 => self.set_sound_timer_to_vx(op_code),
                // FX1E
                0x001E => self.add_vx_to_i(op_code),
                // FX29
                0x0029 => self.set_i_to_sprite(op_code),
                // FX33
                0x0033 => self.bcd(op_code),
                // FX55
                0x0055 => self.reg_dump(op_code),
                // FX65
                0x0065 => self.reg_load(op_code),
                _ => panic!("Unknown opcode [0xF000]: {:#06x}", op_code),
            },
            _ => panic!("Unknown opcode: {}", op_code),
        }
    }
    /** OP Codes  
     * Reference: https://en.wikipedia.org/wiki/CHIP-8#Opcode_table
     */

    /** 0x00E0: Clears the screen */
    fn clear_screen(&mut self) {
        self.gfx.fill(0);
        self.state = ChipState::Clear;
        self.program_counter += 2;
    }

    /** 0x00EE: Returns from subroutine */
    fn return_from_subroutine(&mut self) {
        self.stack_pointer -= 1;
        self.program_counter =
            usize::try_from(self.stack[self.stack_pointer] & 0x0FFF).unwrap() + 2;
    }

    /** 0x1NNN: goto NNN */
    fn goto(&mut self, op_code: u16) {
        self.program_counter = usize::try_from(op_code & 0x0FFF).unwrap();
    }

    /** 2NNN: Calls subroutine at NNN */
    fn call_subroutine(&mut self, op_code: u16) {
        self.stack[self.stack_pointer] = u16::try_from(self.program_counter).unwrap();
        self.stack_pointer += 1;
        self.program_counter = usize::try_from(op_code & 0x0FFF).unwrap();
    }

    /** 3XNN: Skips the next instruction if VX equals NN */
    fn skip_if_eq_to_nn(&mut self, op_code: u16) {
        let x = (usize::try_from(op_code).unwrap() & 0x0F00) >> 8;
        let val = u8::try_from(op_code & 0x00FF).unwrap();
        if self.v[x] == val {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** 4XNN: Skips the next instruction if VX does not equal NN */
    fn skip_if_not_eq_to_nn(&mut self, op_code: u16) {
        let x = (usize::try_from(op_code).unwrap() & 0x0F00) >> 8;
        let val = u8::try_from(op_code & 0x00FF).unwrap();
        if self.v[x] != val {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** 5XY0: Skips the next instruction if VX equals VY */
    fn skip_if_vx_eq_to_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        if self.v[x] == self.v[y] {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** 6XNN: Sets VX to NN */
    fn set_vx_to_nn(&mut self, op_code: u16) {
        let x = (usize::try_from(op_code).unwrap() & 0x0F00) >> 8;
        let val = u8::try_from(op_code & 0x00FF).unwrap();
        self.v[x] = val;
        self.program_counter += 2;
    }

    /** 7XNN: Adds NN to VX */
    fn add_nn_to_vx(&mut self, op_code: u16) {
        let x = (usize::try_from(op_code).unwrap() & 0x0F00) >> 8;
        let val = u8::try_from(op_code & 0x00FF).unwrap();
        self.v[x] += val;
        self.program_counter += 2;
    }

    /** 8XY0: Sets VX to the value of VY */
    fn set_vx_to_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[x] = self.v[y];
        self.program_counter += 2;
    }

    /** 8XY1: Sets VX to VX or VY (bitwise OR operation) */
    fn set_vx_to_vx_or_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[x] |= self.v[y];
        self.program_counter += 2;
    }

    /** 8XY2: Sets VX to VX and VY (bitwise AND operation)*/
    fn set_vx_to_vx_and_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[x] &= self.v[y];
        self.program_counter += 2;
    }

    /** 8XY3: Sets VX to VX xor VY */
    fn set_vx_to_vx_xor_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[x] ^= self.v[y];
        self.program_counter += 2;
    }

    /** 8XY4: Adds VY to VX. VF is set to 1 when there's an overflow, and to 0 when there is not. */
    fn set_vx_to_vx_plus_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[0xF] = if self.v[y] > self.v[x] { 1 } else { 0 };
        self.v[x] += self.v[y];
        self.program_counter += 2;
    }

    /** 8XY5: VY is subtracted from VX. VF is set to 0 when there's an underflow, and 1 when there is not. (i.e. VF set to 1 if VX >= VY and 0 if not) */
    fn set_vx_to_vx_minus_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[0xF] = if self.v[x] > self.v[y] { 1 } else { 0 };
        self.v[x] -= self.v[y];
        self.program_counter += 2;
    }

    /** 8XY6: Stores the least significant bit of VX in VF and then shifts VX to the right by 1 */
    fn shift_right(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.v[0xF] = self.v[x] & 0x01;
        self.v[x] >>= 1;
        self.program_counter += 2;
    }

    /** 8XY7: Sets VX to VY minus VX. VF is set to 0 when there's an underflow, and 1 when there is not. (i.e. VF set to 1 if VY >= VX) */
    fn set_vx_to_vy_minus_vx(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        self.v[0xF] = if self.v[y] > self.v[x] { 1 } else { 0 };
        self.v[x] = self.v[y] - self.v[x];
        self.program_counter += 2;
    }

    /** 8XYE: Stores the most significant bit of VX in VF and then shifts VX to the left by 1 */
    fn shift_left(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.v[0xF] = self.v[x] >> 7;
        self.v[x] <<= 1;
        self.program_counter += 2;
    }

    /** 9XY0: Skips the next instruction if VX does not equal VY */
    fn skip_if_vx_not_eq_vy(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let y = (op_code & 0x00F0) >> 4;
        if self.v[x] != self.v[y] {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** ANNN: Sets I to the address NNN */
    fn set_i_to_nnn(&mut self, op_code: u16) {
        self.i = usize::try_from(op_code & 0x0FFF).unwrap();
        self.program_counter += 2;
    }

    /** BNNN: Jumps to the address NNN plus V0 */
    fn goto_nnn_plus_v0(&mut self, op_code: u16) {
        let val = u16::try_from(op_code & 0x0FFF).unwrap();
        self.program_counter = usize::try_from(self.v[0]).unwrap() + usize::try_from(val).unwrap();
    }

    /** CXNN: Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN */
    fn set_vx_to_rand_and_nn(&mut self, op_code: u16) {
        let val = u8::try_from(op_code & 0x00FF).unwrap();
        let x = usize::try_from((op_code & 0x0F00) >> 8).unwrap();

        self.v[x] = rand::thread_rng().gen_range(0..255) & val;
        self.program_counter += 2;
    }

    /** DXYN: Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels. */
    fn draw(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = self.v[(op_code & 0x0F00) >> 8];
        let y = self.v[(op_code & 0x00F0) >> 4];
        let height = op_code & 0x000F;
        self.v[0x0F] = 0;

        for y_offset in 0..height {
            if height == 0 {
                break;
            }
            let pixel = self.memory[self.i + y_offset];
            for x_offset in 0..8 {
                if (pixel & (0x80 >> u8::try_from(x_offset).unwrap())) != 0 {
                    let x = usize::try_from(x).unwrap();
                    let y = usize::try_from(y).unwrap();
                    let index = x + x_offset + ((y + y_offset) * 64);
                    if self.gfx[index] == 1 {
                        self.v[0x0F] = 1;
                    }
                    self.gfx[index] ^= 1;
                }
            }
        }

        self.state = ChipState::Draw;

        self.program_counter += 2;
    }

    /** EX9E: Skips the next instruction if the key stored in VX is pressed */
    fn skip_if_key_pressed(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let index = usize::try_from(self.v[x]).unwrap();
        if self.keys[index] != 0 {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** EXA1: Skips the next instruction if the key stored in VX is not pressed */
    fn skip_if_not_key_pressed(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        let index = usize::try_from(self.v[x]).unwrap();
        if self.keys[index] == 0 {
            self.program_counter += 4;
        } else {
            self.program_counter += 2;
        }
    }

    /** FX07: Sets VX to the value of the delay timer */
    fn set_vx_to_delay_timer(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.v[x] = self.delay_timer;
        self.program_counter += 2;
    }

    /** Helper for FX0A  */
    fn is_key_press(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.state = ChipState::Block;
        for i in 0..16 {
            if self.keys[i] == 1 {
                println!("FX0A key {} was pressed", i);
                self.v[x] = u8::try_from(i).unwrap();
                self.state = ChipState::Run;
                break;
            }
        }

        if self.state == ChipState::Run {
            self.program_counter += 2;
        }
    }

    /** FX15: Sets the delay timer to VX */
    fn set_delay_timer_to_vx(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.delay_timer = self.v[x];
        self.program_counter += 2;
    }

    /**  FX18: Sets the sound timer to VX */
    fn set_sound_timer_to_vx(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.sound_timer = self.v[x];
        self.program_counter += 2;
    }

    /** FX1E: Adds VX to I. VF is not affected */
    fn add_vx_to_i(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.i += usize::try_from(self.v[x]).unwrap();
        self.program_counter += 2;
    }

    /** FX29: Sets I to the location of the sprite for the character in VX */
    fn set_i_to_sprite(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        self.i = usize::try_from(self.v[x]).unwrap() * 0x5;
        self.program_counter += 2;
    }

    /** FX33: Stores the binary-coded decimal representation of VX, with the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2 */
    fn bcd(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        self.memory[self.i] = self.v[(op_code & 0x0F00) >> 8] / 100;
        self.memory[self.i + 1] = (self.v[(op_code & 0x0F00) >> 8] / 10) % 10;
        self.memory[self.i + 2] = (self.v[(op_code & 0x0F00) >> 8] % 100) % 10;
        self.program_counter += 2;
    }

    /**  FX55: Stores from V0 to VX (including VX) in memory, starting at address I. The offset from I is increased by 1 for each value written, but I itself is left unmodified */
    fn reg_dump(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        for n in 0..(x + 1) {
            self.memory[self.i + n] = self.v[n]
        }
        self.program_counter += 2;
    }

    /** FX65: Fills from V0 to VX (including VX) with values from memory, starting at address I. The offset from I is increased by 1 for each value read, but I itself is left unmodified */
    fn reg_load(&mut self, op_code: u16) {
        let op_code = usize::try_from(op_code).unwrap();
        let x = (op_code & 0x0F00) >> 8;
        for n in 0..(x + 1) {
            self.v[n] = self.memory[self.i + n]
        }
        self.program_counter += 2;
    }
}

#[allow(arithmetic_overflow)]
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn init_state() {
        let chip8 = Chip8::new();
        let mut mem = [0u8; 4096];

        for (place, data) in mem.iter_mut().zip(CHIP8_FONT_SET.iter()) {
            *place = *data
        }

        assert_eq!(chip8.memory, mem);
        assert_eq!(chip8.v, [0u8; 16]);
        assert_eq!(chip8.program_counter, 512);
        assert_eq!(chip8.gfx, [0u8; 64 * 32]);
        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.delay_timer, 0);
        assert_eq!(chip8.sound_timer, 0);
        assert_eq!(chip8.stack, [0u16; 16]);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.keys, [0u8; 16]);
    }

    #[test]
    fn get_op_code() {
        let mut chip8 = Chip8::new();
        let program_counter = 0x210;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0xfa;
        chip8.memory[program_counter + 1] = 0x1e;

        assert_eq!(chip8.get_op_code(), 0xfa1e);
    }

    #[test]
    fn op_code_00_e0_clear_screen() {
        let mut chip8 = Chip8::new();
        let program_counter = 0x210;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x00;
        chip8.memory[program_counter + 1] = 0xe0;
        chip8.gfx[2] = 1;
        chip8.gfx[33] = 1;
        chip8.gfx[444] = 1;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Clear);
        assert_eq!(chip8.gfx, [0u8; 64 * 32]);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.stack, [0; 16]);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_00_ee_return_from_subroutine() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x212;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x00;
        chip8.memory[program_counter + 1] = 0xee;

        chip8.stack[chip8.stack_pointer] = 0x321;
        chip8.stack_pointer += 1;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, 0x321 + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_1n_nn_goto() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x214;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x14;
        chip8.memory[program_counter + 1] = 0x32;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, 0x432);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_2n_nn_call_subroutine() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x216;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x24;
        chip8.memory[program_counter + 1] = 0x36;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, 0x436);
        assert_eq!(chip8.stack_pointer, 1);
        assert_eq!(chip8.stack[0], 0x216);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_3x_nn_skip_if_eq_to_nn_true() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x218;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x3a;
        chip8.memory[program_counter + 1] = 0x56;
        chip8.v[0x0a] = 0x56;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 4);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_3x_nn_skip_if_eq_to_nn_false() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x220;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x3b;
        chip8.memory[program_counter + 1] = 0x1f;
        chip8.v[0x0b] = 0x2f;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_4x_nn_skip_if_not_eq_to_nn_true() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x222;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x44;
        chip8.memory[program_counter + 1] = 0x1a;
        chip8.v[0x04] = 0x2a;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 4);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_4x_nn_skip_if_not_eq_to_nn_false() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x224;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x4f;
        chip8.memory[program_counter + 1] = 0x33;
        chip8.v[0x0f] = 0x33;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_5x_y0_skip_if_vx_eq_to_vy_true() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x224;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x52;
        chip8.memory[program_counter + 1] = 0xe0;
        chip8.v[0x02] = 0x33;
        chip8.v[0x0e] = 0x33;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 4);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_5x_y0_skip_if_vx_eq_to_vy_false() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x224;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x53;
        chip8.memory[program_counter + 1] = 0xa0;
        chip8.v[0x03] = 0x55;
        chip8.v[0x0a] = 0xaa;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
    }

    #[test]
    fn op_code_6x_nn_set_vx_to_nn() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x224;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x6d;
        chip8.memory[program_counter + 1] = 0x1a;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x0d], 0x1a);
    }

    #[test]
    fn op_code_7x_nn_add_nn_to_vx() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x224;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x7c;
        chip8.memory[program_counter + 1] = 0x29;
        chip8.v[0x0c] = 0x3a;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x0c], 0x3a + 0x29);
    }

    #[test]
    fn op_code_7x_nn_add_nn_to_vx_overflow() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x226;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x7c;
        chip8.memory[program_counter + 1] = 0xff;
        chip8.v[0x0c] = 0x3a;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x0c], 0x3a + 0xff);
        // carry flag is not changed
        assert_eq!(chip8.v[0x0f], 0x00);
    }

    #[test]
    fn op_code_8x_y0_set_vx_to_vy() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x228;
        let vx = 0x11;
        let vy = 0x4d;

        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x80;
        chip8.memory[program_counter + 1] = 0x50;
        chip8.v[0x00] = vx;
        chip8.v[0x05] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x00], vy);
    }

    #[test]
    fn op_code_8x_y1_set_vx_to_vx_or_vy() {
        let mut chip8 = Chip8::new();
        let vx = 0x23;
        let vy = 0x6a;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x81;
        chip8.memory[program_counter + 1] = 0x41;
        chip8.v[0x01] = vx;
        chip8.v[0x04] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x01], vx | vy);
    }

    #[test]
    fn op_code_8x_y2_set_vx_to_vx_and_vy() {
        let mut chip8 = Chip8::new();
        let vx = 0x32;
        let vy = 0xe5;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x84;
        chip8.memory[program_counter + 1] = 0x82;
        chip8.v[0x04] = vx;
        chip8.v[0x08] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x04], vx & vy);
    }

    #[test]
    fn op_code_8x_y3_set_vx_to_vx_xor_vy() {
        let mut chip8 = Chip8::new();
        let vx = 0x46;
        let vy = 0x4f;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x85;
        chip8.memory[program_counter + 1] = 0xc3;
        chip8.v[0x05] = vx;
        chip8.v[0x0c] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x05], vx ^ vy);
    }

    #[test]
    fn op_code_8x_y4_set_vx_to_vx_plus_vy() {
        let mut chip8 = Chip8::new();
        let vx = 0x65;
        let vy = 0x1d;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x82;
        chip8.memory[program_counter + 1] = 0x64;
        chip8.v[0x02] = vx;
        chip8.v[0x06] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x02], vx + vy);
        assert_eq!(chip8.v[0x0f], 0);
    }

    #[test]
    fn op_code_8x_y4_set_vx_to_vx_plus_vy_overflow() {
        let mut chip8 = Chip8::new();
        let vx = 0xd1;
        let vy = 0xe2;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x83;
        chip8.memory[program_counter + 1] = 0xd4;
        chip8.v[0x03] = vx;
        chip8.v[0x0d] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x03], vx + vy);
        assert_eq!(chip8.v[0x0f], 1);
    }

    #[test]
    fn op_code_8x_y5_set_vx_to_vx_minus_vy() {
        let mut chip8 = Chip8::new();
        let vx = 0x43;
        let vy = 0xd2;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x80;
        chip8.memory[program_counter + 1] = 0x95;
        chip8.v[0x00] = vx;
        chip8.v[0x09] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x00], vx - vy);
        assert_eq!(chip8.v[0x0f], 0);
    }

    #[test]
    fn op_code_8x_y5_set_vx_to_vx_minus_vy_overflow() {
        let mut chip8 = Chip8::new();
        let vx = 0xf3;
        let vy = 0x32;

        let program_counter = 0x230;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x83;
        chip8.memory[program_counter + 1] = 0xc5;
        chip8.v[0x03] = vx;
        chip8.v[0x0c] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x03], vx - vy);
        assert_eq!(chip8.v[0x0f], 1);
    }

    #[test]
    fn op_code_8x_y6_shift_right() {
        let mut chip8 = Chip8::new();
        let vx = 0xaf;

        let program_counter = 0x232;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x82;
        chip8.memory[program_counter + 1] = 0xc6;
        chip8.v[0x02] = vx;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x02], 0x57);
        assert_eq!(chip8.v[0x0f], 0x01);
    }

    #[test]
    fn op_code_8x_y7_set_vx_to_vy_minus_vx() {
        let mut chip8 = Chip8::new();
        let vx = 0xaf;
        let vy = 0x32;

        let program_counter = 0x234;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x8a;
        chip8.memory[program_counter + 1] = 0xc7;
        chip8.v[0x0a] = vx;
        chip8.v[0x0c] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x0a], vy - vx);
        assert_eq!(chip8.v[0x0c], vy);
        assert_eq!(chip8.v[0x0f], 0x00);
    }

    #[test]
    fn op_code_8x_y7_set_vx_to_vy_minus_vx_overflow() {
        let mut chip8 = Chip8::new();
        let vx = 0x34;
        let vy = 0xaa;

        let program_counter = 0x236;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x8a;
        chip8.memory[program_counter + 1] = 0xc7;
        chip8.v[0x0a] = vx;
        chip8.v[0x0c] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x0a], vy - vx);
        assert_eq!(chip8.v[0x0c], vy);
        assert_eq!(chip8.v[0x0f], 0x01);
    }

    #[test]
    fn op_code_8x_ye_shift_left() {
        let mut chip8 = Chip8::new();
        let vx = 0xae;

        let program_counter = 0x238;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x85;
        chip8.memory[program_counter + 1] = 0xce;
        chip8.v[0x05] = vx;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x05], 0x5c);
        assert_eq!(chip8.v[0x0f], 0x01);
    }

    #[test]
    fn op_code_9x_y0_skip_if_vx_not_eq_vy_true() {
        let mut chip8 = Chip8::new();
        let vx = 0x3e;
        let vy = 0x3e;

        let program_counter = 0x240;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x93;
        chip8.memory[program_counter + 1] = 0x60;
        chip8.v[0x03] = vx;
        chip8.v[0x06] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x03], vx);
        assert_eq!(chip8.v[0x06], vy);
    }

    #[test]
    fn op_code_9x_y0_skip_if_vx_not_eq_vy_false() {
        let mut chip8 = Chip8::new();
        let vx = 0x3e;
        let vy = 0x0a;

        let program_counter = 0x240;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0x93;
        chip8.memory[program_counter + 1] = 0x60;
        chip8.v[0x03] = vx;
        chip8.v[0x06] = vy;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 4);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x03], vx);
        assert_eq!(chip8.v[0x06], vy);
    }

    #[test]
    fn op_code_an_nn_set_i_to_nnn() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x240;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0xa0;
        chip8.memory[program_counter + 1] = 0x63;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, program_counter + 2);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0x063);
    }

    #[test]
    fn op_code_bn_nn_goto_nnn_plus_v0() {
        let mut chip8 = Chip8::new();

        let program_counter = 0x240;
        chip8.program_counter = program_counter;
        chip8.memory[program_counter] = 0xb2;
        chip8.memory[program_counter + 1] = 0x63;
        chip8.v[0x00] = 0x12;

        chip8.execute();

        assert_eq!(chip8.state, ChipState::Run);
        assert_eq!(chip8.program_counter, 0x12 + 0x263);
        assert_eq!(chip8.stack_pointer, 0);
        assert_eq!(chip8.i, 0);
        assert_eq!(chip8.v[0x00], 0x12);
    }

    // #[test]
    // fn op_code_dx_yn_draw() {
    //     let mut chip8 = Chip8::new();
    //     let vx = 0x3e;
    //     let vy = 0x0a;

    //     let program_counter = 0x240;
    //     chip8.program_counter = program_counter;
    //     chip8.memory[program_counter] = 0xc0;
    //     chip8.memory[program_counter + 1] = 0x11;
    //     chip8.v[0x00] = vx;
    //     chip8.v[0x01] = vy;

    //     chip8.execute();

    //     let mut gfx_expected = [0; 64 * 32];

    //     assert_eq!(chip8.state, ChipState::Run);
    //     assert_eq!(chip8.program_counter, 0x12 + 0x263);
    //     assert_eq!(chip8.stack_pointer, 0);
    //     assert_eq!(chip8.i, 0);
    //     assert_eq!(chip8.v[0x00], 0x12);
    // }
}
