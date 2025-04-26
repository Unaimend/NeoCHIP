struct CPU {
  // If position_in_memory is > 0x1000 we have a problem.
  position_in_memory: usize,
  registers: [u8; 16],
  // We do not reserer 512 bytes for the system
  memory: [u8; 0x1000],
  stack: [u16; 16],
  // allows us to panick the stack
  stack_pointer: usize,
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
      let nnn = opcode & 0x0FFF;
      match (c, x, y, d) {
        (0, 0, 0, 0) => {
          return;
        }
        (0, 0, 0xE, 0xE) => self.ret(),
        (0x2, _, _, _) => self.call(nnn), //TODO is this into correct?
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
}

fn main() {
  let mut cpu = CPU {
    registers: [0; 16],
    memory: [0; 4096],
    position_in_memory: 0,
    stack: [0; 16],
    stack_pointer: 0,
  };

  cpu.registers[0] = 5;
  cpu.registers[1] = 10;

  let mem = &mut cpu.memory;
  let add_twice = [0x80, 0x14, 0x80, 0x14, 0x00, 0xEE];
  mem[0x100..0x106].copy_from_slice(&add_twice);
  //cpu.position_in_memory = 0x100;
  //cpu.stack[0] = 0x000;
  //cpu.stack_pointer += 1;
  mem[0x000] = 0x21;
  mem[0x001] = 0x00;
  mem[0x002] = 0x21;
  mem[0x003] = 0x00;
  mem[0x004] = 0x00;
  mem[0x005] = 0x00;
  println!("{:?}", cpu.registers);
  cpu.run();
  println!("{:?}", cpu.registers);
}
