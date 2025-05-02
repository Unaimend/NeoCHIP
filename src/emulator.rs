// [1] http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#8xy5
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[allow(unused)]
fn main() {
  let path = Path::new("hello.bin");
  let display = path.display();

  // Open the path in read-only mode, returns `io::Result<File>`
  let mut file = match File::open(path) {
    Err(why) => panic!("couldn't open {display}: {why}"),
    Ok(file) => file,
  };

  let mut cpu = CPU {
    registers: [0; 16],
    register_i: 0,
    memory: [0; 4096],
    position_in_memory: 0,
    stack: [0; 16],
    stack_pointer: 0,
  };

  file.read(&mut cpu.memory).expect("Error reading file");

  println!("{:?}", cpu.memory);
}
struct CPU {
  // If position_in_memory is > 0x1000 we have a problem.
  position_in_memory: usize,
  registers: [u8; 16],
  register_i: u16,
  // We do not reserer 512 bytes for the system
  memory: [u8; 0x1000],
  stack: [u16; 16],
  // allows us to panick the stack
  stack_pointer: usize,
}

impl CPU {
  //nnn or addr - A 12-bit value, the lowest 12 bits of the instruction
  //n or nibble - A 4-bit value, the lowest 4 bits of the instruction
  //x - A 4-bit value, the lower 4 bits of the high byte of the instruction
  //y - A 4-bit value, the upper 4 bits of the low byte of the instruction
  //kk or byte - An 8-bit value, the lowest 8 bits of the instruction

  fn read_opcode(&self) -> u16 {
    let p = self.position_in_memory;
    // Why u16? When we shift below, we would just zero
    // op_byte1 if it were u8
    // xxxxxxxx
    // yyyyyyyy
    // yyyyyyyy as u 16 = 00000000yyyyyyyy
    // xxxxxxxx << 8    = xxxxxxxx000000000
    // xxxxxxxx00000000 | 00000000yyyyyyyy = xxxxxxxxyyyyyyyy
    let op_byte1 = self.memory[p] as u16;
    let op_byte2 = self.memory[p + 1] as u16;
    (op_byte1 << 8) | op_byte2
  }
  fn step(&mut self) -> i32 {
    let opcode = self.read_opcode();
    self.position_in_memory += 2;
    // Extract 4Bit and shit to lowest pos.
    let c = ((opcode & 0xF000) >> 12) as u8;
    let x = ((opcode & 0x0F00) >> 8) as u8;
    let y = ((opcode & 0x00F0) >> 4) as u8;
    let d = (opcode & 0x000F) as u8;
    let nnn = opcode & 0x0FFF;
    // The the lower byte
    let kk = (opcode & 0x00FF) as u8;
    match (c, x, y, d) {
      (0, 0, 0, 0) => {
        return 0;
      }
      (0, 0, 0xE, 0xE) => self.ret(),
      (0x1, _, _, _) => self.jmp(nnn),
      (0x2, _, _, _) => self.call(nnn),
      (0x3, _, _, _) => self.skip_equal(x, kk),
      (0x4, _, _, _) => self.skip_not_equal(x, kk),
      (0x5, _, _, _) => self.skip_reg_equal(x, y),
      (0x6, _, _, _) => self.load_immediate(x, kk),
      (0x7, _, _, _) => self.add_x_immediate(x, kk),
      (0x8, _, _, 0x0) => self.store_y_in_x(x, y),
      (0x8, _, _, 0x1) => self.or(x, y),
      (0x8, _, _, 0x2) => self.and(x, y),
      (0x8, _, _, 0x3) => self.xor(x, y),
      (0x8, _, _, 0x4) => self.add_xy(x, y),
      (0x8, _, _, 0x5) => self.subs_xy(x, y),
      (0x9, _, _, _) => self.skip_reg_not_equal(x, y),
      (0xA, _, _, _) => self.set_register_i(nnn),
      //(0xB, _, _, _) => todo!("Implement B"),
      //(0xC, _, _, _) => todo!("Implement rand"),
      //(0xD, _, _, _) => todo!("Implement display"),
      //(0xE, _, _, _) => todo!("Implement keyboard"),
      (0xF, _, 0x5, 0x5) => self.store_regs(x),
      (0xF, _, 0x6, 0x5) => self.load_regs(x),
      _ => todo!("opcode {:04x}", opcode),
    }
    1
  }

  fn run(&mut self) {
    loop {
      if self.step() == 0 {
        break;
      }
    }
  }

  fn or(&mut self, x: u8, y: u8) {
    let arg1 = self.registers[x as usize];
    let arg2 = self.registers[y as usize];
    self.registers[x as usize] = arg1 | arg2;
  }

  fn and(&mut self, x: u8, y: u8) {
    let arg1 = self.registers[x as usize];
    let arg2 = self.registers[y as usize];
    self.registers[x as usize] = arg1 & arg2;
  }

  fn xor(&mut self, x: u8, y: u8) {
    let arg1 = self.registers[x as usize];
    let arg2 = self.registers[y as usize];
    self.registers[x as usize] = arg1 ^ arg2;
  }

  fn add_xy(&mut self, x: u8, y: u8) {
    let arg1 = self.registers[x as usize];
    let arg2 = self.registers[y as usize];
    let (val, overflow) = arg1.overflowing_add(arg2);
    self.registers[x as usize] = val;

    if overflow {
      self.registers[0xF] = 1;
    } else {
      self.registers[0xF] = 0;
    }
  }

  fn subs_xy(&mut self, x: u8, y: u8) {
    let arg1 = self.registers[x as usize];
    let arg2 = self.registers[y as usize];

    //TODO not sure about this?
    if arg1 >= arg2 {
      self.registers[x as usize] = arg1 - arg2;
      self.registers[0xF] = 1;
    } else {
      self.registers[0xF] = 0;
    }
  }

  fn add_x_immediate(&mut self, x: u8, kk: u8) {
    let arg1 = self.registers[x as usize];
    let (val, overflow) = arg1.overflowing_add(kk);
    self.registers[x as usize] = val;
    // TODO [1] does not mention overflow behaviour
    if overflow {
      self.registers[0xF] = 1;
    } else {
      self.registers[0xF] = 0;
    }
  }

  fn store_y_in_x(&mut self, x: u8, y: u8) {
    self.registers[x as usize] = self.registers[y as usize];
  }

  fn set_register_i(&mut self, nnn: u16) {
    self.register_i = nnn;
  }

  fn store_regs(&mut self, x: u8) {
    let mut start_loc = self.register_i;

    for reg in 0..x + 1 {
      match self.registers.get(reg as usize) {
        Some(val) => self.memory[(start_loc) as usize] = *val,
        None => panic!("Register not found"),
      }
      start_loc += 1;
    }
  }

  fn load_regs(&mut self, x: u8) {
    let mut start_loc = self.register_i;

    for reg in 0..x + 1 {
      match self.registers.get_mut(reg as usize) {
        Some(val) => {
          *val = self.memory[(start_loc) as usize];
        }
        None => panic!("Register not found"),
      }
      start_loc += 1;
    }
  }

  fn jmp(&mut self, addr: u16) {
    self.position_in_memory = addr as usize;
  }

  fn call(&mut self, addr: u16) {
    let sp = self.stack_pointer;
    let stack = &mut self.stack;
    if sp >= stack.len() {
      panic!("Stack overflow");
    }
    stack[sp] = self.position_in_memory as u16;
    self.stack_pointer += 1;
    self.position_in_memory = addr as usize;
  }

  fn ret(&mut self) {
    if self.stack_pointer == 0 {
      panic!("Stack underflow");
    }
    self.stack_pointer -= 1;
    let call_addr = self.stack[self.stack_pointer];
    self.position_in_memory = call_addr as usize;
  }

  fn skip_equal(&mut self, x: u8, kk: u8) {
    if self.registers[x as usize] == kk {
      self.position_in_memory += 2;
    }
  }

  fn skip_not_equal(&mut self, x: u8, kk: u8) {
    if self.registers[x as usize] != kk {
      self.position_in_memory += 2;
    }
  }

  fn skip_reg_equal(&mut self, x: u8, y: u8) {
    if self.registers[x as usize] == self.registers[y as usize] {
      self.position_in_memory += 2;
    }
  }

  fn skip_reg_not_equal(&mut self, x: u8, y: u8) {
    if self.registers[x as usize] != self.registers[y as usize] {
      self.position_in_memory += 2;
    }
  }

  fn load_immediate(&mut self, x: u8, kk: u8) {
    self.registers[x as usize] = kk;
  }
}

fn run(mut cpu: CPU, program: &[u8], n: usize) {
  for i in 0..n {
    if cpu.step() == 0 {
      break;
    }
    println!(
      "i: {:04} | pc: {:04} {:?}| 0x{:02X}{:02X}",
      i,
      cpu.position_in_memory,
      cpu.registers,
      program[cpu.position_in_memory],
      program[cpu.position_in_memory + 1]
    );
  }
}

#[cfg(test)]
mod tests {
  // We need to bring the CPU struct into the scope of our tests module
  use super::*;

  #[test]
  fn subtract() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x000, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    let program: [u8; 6] = [
      // Main program (starts at 0x200)
      0x61, 0x01, // 0x200: LD V0, 0x01
      0x60, 0x02, // 0x202: LD V1, 0x02
      0x80, 0x15, // 0x204: SUB V1, V0
    ];

    cpu.memory[0..6].copy_from_slice(&program);

    cpu.run();
    assert_eq!(cpu.registers[0], 1);
  }

  #[test]
  fn subtract2() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x000, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    let program = [
      // Main program (starts at 0x200)
      0x60, 5, // 0x200: LD V0, 0x01
      0x61, 10, // 0x202: LD V1, 0x02
      0x81, 0x05, // 0x204: SUB V1, V0
    ];

    cpu.memory[0..6].copy_from_slice(&program);

    cpu.run();
    assert_eq!(cpu.registers[1], 5);
  }

  #[test]
  fn fibonacci_manual() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x000, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    let program: [u8; 6] = [
      // Main program (starts at 0x200)
      0x60, 0x01, // 0x200: LD V0, 0x01
      0x61, 0x01, // 0x202: LD V1, 0x02
      0x80, 0x14, // 0x204: ADD V0, V1
    ];

    cpu.memory[0..6].copy_from_slice(&program);

    cpu.run();
    assert_eq!(cpu.registers[0], 2);
  }

  #[test]
  fn fibonacci_loop() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x000, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    //f_n+1 = f_n + n_n-1

    const LEN: usize = 22;
    let program: [u8; LEN] = [
      // Main program (starts at 0x200)
      0x60, 0x01, // 0x002: LD V0, 0x01
      0x61, 0x01, // 0x004: LD V1, 0x01
      0x65, 0x05, // 0x006: LD V5, 0x05
      0x66, 0x01, // 0x008: LD V6, 0x01
      0x85, 0x65, // 0x00A: V5 <- V5 - V6 // Decrement counter
      0x83, 0x00, //      : V3 <- V0      // Save f_n+1
      0x80, 0x14, // 0x00C: ADD V0, V1
      0x81, 0x30, //      : V1 <- V3
      0x35, 0x00, //      : V5 == 0;
      0x10, 0x08, // JMP 0x008
      0x00, 0x00,
    ];

    cpu.memory[0..LEN].copy_from_slice(&program);
    cpu.run();
    assert_eq!(cpu.registers[0], 13);
  }

  #[test]
  // Test case where the conditional skip happens and a subroutine is called
  fn test_program_skip_and_call() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x200, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    // Program bytes:
    // - Load 10 into V0
    // - Load 5 into V1
    // - Add V1 to V0 (V0 becomes 15)
    // - XOR V0 with V1 (V0 becomes 10)
    // - Skip next instruction if V0 == 10 (V0 is 10, so skip the JMP)
    // - JMP 0x214 (This is skipped)
    // - CALL subroutine at 0x216
    // - Load 255 into V3 (after returning from subroutine)
    // - AND V0 with V3 (V0 remains 10)
    // - Load 1 into V4
    // - HALT
    // - Subroutine at 0x216:
    //   - Load 0 into V2
    //   - Add 20 to V2 (V2 becomes 20)
    //   - Load 170 (0xAA) into V5
    //   - Return
    let program: [u8; 30] = [
      // Main program (starts at 0x200)
      0x60, 0x0A, // 0x200: LD V0, 0x0A
      0x61, 0x05, // 0x202: LD V1, 0x05
      0x80, 0x14, // 0x204: ADD V0, V1 (V0 = 15)
      0x80, 0x13, // 0x206: XOR V0, V1 (V0 = 10)
      0x30, 0x0A, // 0x208: SE V0, 0x0A (Skip next if V0 == 10)
      0x12, 0x14, // 0x20A: JMP 0x214 (Skipped)
      0x22, 0x16, // 0x20C: CALL 0x216
      0x63, 0xFF, // 0x20E: LD V3, 0xFF
      0x80, 0x32, // 0x210: AND V0, V3
      0x64, 0x01, // 0x212: LD V4, 1
      0x00, 0x00, // 0x214: HALT
      // Subroutine (starts at 0x216)
      0x62, 0x00, // 0x216: LD V2, 0x00
      0x72, 0x14, // 0x218: ADD V2, 0x14 (V2 = 20)
      0x65, 0xAA, // 0x21A: LD V5, 0xAA
      0x00, 0xEE, // 0x21C: RET
    ];

    // Load the program into memory at 0x200
    let program_start_address = 0x200;
    let program_end_address = program_start_address + program.len();
    assert!(
      program_end_address <= cpu.memory.len(),
      "Program exceeds memory capacity"
    );
    cpu.memory[program_start_address..program_end_address].copy_from_slice(&program);

    // Run the CPU
    cpu.run();

    // Assert the expected final state of key registers
    assert_eq!(cpu.registers[0], 10, "V0 should be 10");
    assert_eq!(cpu.registers[1], 5, "V1 should be 5");
    assert_eq!(
      cpu.registers[2], 20,
      "V2 should be 20 (modified by subroutine)"
    );
    assert_eq!(cpu.registers[3], 255, "V3 should be 255");
    assert_eq!(cpu.registers[4], 1, "V4 should be 1");
    assert_eq!(
      cpu.registers[5], 170,
      "V5 should be 170 (modified by subroutine)"
    );
    assert_eq!(cpu.registers[0xF], 0, "VF should be 0 (no carry from ADD)");

    // Verify other registers remain at their default (0)
    for i in 6..15 {
      assert_eq!(cpu.registers[i], 0, "V{i} should be 0");
    }
  }

  #[test]
  // Test case where the conditional skip does NOT happen and a jump is executed
  fn test_program_no_skip_and_jump() {
    let mut cpu = CPU {
      registers: [0; 16],
      register_i: 0,
      memory: [0; 4096],
      position_in_memory: 0x200, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    // Program bytes:
    // - Load 11 into V0 (Changed from 10 in the first test)
    // - Load 5 into V1
    // - Add V1 to V0 (V0 becomes 16)
    // - XOR V0 with V1 (V0 becomes 21)
    // - Skip next instruction if V0 == 10 (V0 is 21, so DO NOT skip)
    // - JMP 0x214 (This is executed)
    // - CALL subroutine at 0x216 (Skipped by JMP)
    // - Load 255 into V3 (Skipped by JMP)
    // - AND V0 with V3 (Skipped by JMP)
    // - Load 1 into V4 (Skipped by JMP)
    // - HALT
    // - Subroutine at 0x216 (Not reached):
    //   - Load 0 into V2
    //   - Add 20 to V2
    //   - Load 170 (0xAA) into V5
    //   - Return
    let program: [u8; 30] = [
      // Main program (starts at 0x200)
      0x60, 0x0B, // 0x200: LD V0, 0x0B (V0 = 11) - Changed!
      0x61, 0x05, // 0x202: LD V1, 0x05
      0x80, 0x14, // 0x204: ADD V0, V1 (V0 = 16)
      0x80, 0x13, // 0x206: XOR V0, V1 (V0 = 21)
      0x30, 0x0A, // 0x208: SE V0, 0x0A (Skip next if V0 == 10) - V0 is 21, NO skip
      0x12, 0x14, // 0x20A: JMP 0x214 (Executed)
      0x22, 0x16, // 0x20C: CALL 0x216 (Skipped)
      0x63, 0xFF, // 0x20E: LD V3, 0xFF (Skipped)
      0x80, 0x32, // 0x210: AND V0, V3 (Skipped)
      0x64, 0x01, // 0x212: LD V4, 1 (Skipped)
      0x00, 0x00, // 0x214: HALT
      // Subroutine (starts at 0x216 - Not reached)
      0x62, 0x00, // 0x216: LD V2, 0x00
      0x72, 0x14, // 0x218: ADD V2, 0x14
      0x65, 0xAA, // 0x21A: LD V5, 0xAA
      0x00, 0xEE, // 0x21C: RET
    ];

    // Load the program into memory at 0x200
    let program_start_address = 0x200;
    let program_end_address = program_start_address + program.len();
    assert!(
      program_end_address <= cpu.memory.len(),
      "Program exceeds memory capacity"
    );
    cpu.memory[program_start_address..program_end_address].copy_from_slice(&program);

    // Run the CPU
    cpu.run();

    // Assert the expected final state of key registers
    assert_eq!(cpu.registers[0], 21, "V0 should be 21");
    assert_eq!(cpu.registers[1], 5, "V1 should be 5");
    assert_eq!(cpu.registers[2], 0, "V2 should be 0 (subroutine skipped)");
    assert_eq!(cpu.registers[3], 0, "V3 should be 0 (skipped by jump)");
    assert_eq!(cpu.registers[4], 0, "V4 should be 0 (skipped by jump)");
    assert_eq!(cpu.registers[5], 0, "V5 should be 0 (subroutine skipped)");
    assert_eq!(cpu.registers[0xF], 0, "VF should be 0 (no carry from ADD)");

    // Verify other registers remain at their default (0)
    for i in 6..15 {
      assert_eq!(cpu.registers[i], 0, "V{i} should be 0");
    }
  }
}
