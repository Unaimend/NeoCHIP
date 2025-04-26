struct CPU {
  // If position_in_memory is > 0x1000 we have a problem.
  position_in_memory: usize,
  registers: [u8; 16],
  // We do not reserer 512 bytes for the system
  memory: [u8; 0x1000],
}

impl CPU {
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

  fn run(&mut self) {
    loop {
      let opcode = self.read_opcode();
      self.position_in_memory += 2;
      // Extract 4Bit and shit to lowest pos.
      let c = ((opcode & 0xF000) >> 12) as u8;
      let x = ((opcode & 0x0F00) >> 8) as u8;
      let y = ((opcode & 0x00F0) >> 4) as u8;
      let d = ((opcode & 0x000F) >> 0) as u8;

      match (c, x, y, d) {
        (0, 0, 0, 0) => {
          return;
        }
        (0x8, _, _, 0x4) => self.add_xy(x, y),
        _ => todo!("opcode {:04x}", opcode),
      }
    }
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
}

fn main() {
  let mut cpu = CPU {
    registers: [0; 16],
    memory: [0; 4096],
    position_in_memory: 0,
  };

  cpu.registers[0] = 5;
  cpu.registers[1] = 10;
  cpu.registers[2] = 10;
  cpu.registers[3] = 10;

  let mem = &mut cpu.memory;

  mem[0] = 0x80;
  mem[1] = 0x14;
  mem[2] = 0x80;
  mem[3] = 0x34;
  mem[4] = 0x80;
  mem[5] = 0x34;

  cpu.run();
  println!("{:?}", cpu.registers);
}
