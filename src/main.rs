// [1] http://devernay.free.fr/hacks/chip8/C8TECH10.HTM#8xy5
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "assembler.pest"]
pub struct AssemblerParser;

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
    let d = ((opcode & 0x000F) >> 0) as u8;
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
      //(0xA, _, _, _) => todo!("Implement A"),
      //(0xB, _, _, _) => todo!("Implement B"),
      //(0xC, _, _, _) => todo!("Implement rand"),
      //(0xD, _, _, _) => todo!("Implement display"),
      //(0xE, _, _, _) => todo!("Implement keyboard"),
      _ => todo!("opcode {:04x}", opcode),
    }
    return 1;
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
use pest::iterators::{Pair, Pairs};
use std::str::FromStr; // Needed for parse::<T>()

// Assume your Rule enum and AssemblerParser struct are defined and generated by Pest
// #[derive(Parser)]
// #[grammar = "path/to/your_grammar.pest"] // Specify the path to your pest file
// struct AssemblerParser;

// #[derive(Rule_Helper)] // If you use rule_helper
// enum Rule { ... }

// Define data structures to represent your instructions
#[derive(Debug)]
struct Instruction {
  mnemonic: String,
  operand1: Option<Operand>,
  operand2: Option<Operand>,
}

#[derive(Debug)]
enum Operand {
  Register(u8),
  Immediate(i32), // Use i32 for potentially signed numbers from your 'number' rule
  Address(u16),
}

// --- Helper functions to parse operand strings ---

// Parses a register string like "R1" or "R10"
fn parse_register(reg_str: &str) -> Result<u8, String> {
  if !reg_str.starts_with('R') {
    return Err(format!(
      "Invalid register format: must start with 'R', got '{}'",
      reg_str
    ));
  }
  reg_str[1..]
    .parse::<u8>() // Parse the number part after 'R'
    .map_err(|e| format!("Invalid register number '{}': {}", reg_str, e))
}

// Parses a number string (assuming decimal for simplicity, needs extension for hex)
fn parse_number(num_str: &str) -> Result<i32, String> {
  // Your 'number' rule allows a sign and uses your 'positive' rule.
  // This simple parse handles decimal signed integers.
  // You would need to add logic here to handle hexadecimal if allowed by your 'number' rule
  // (e.g., check for "0x" prefix and parse accordingly).
  i32::from_str(num_str).map_err(|e| format!("Invalid number format '{}': {}", num_str, e))
}

// Parses an address string (assuming decimal for simplicity, needs extension for hex)
fn parse_address(addr_str: &str) -> Result<u16, String> {
  // Your 'address' rule defines the valid range 0-4096 and patterns.
  // This simple parse handles decimal.
  // You would need to add logic here to handle hexadecimal addresses like 0x008 if allowed.
  u16::from_str(addr_str).map_err(|e| format!("Invalid address format '{}': {}", addr_str, e))
}

// --- Main function to process the parse tree ---

fn process_parse_tree(pairs: Pairs<Rule>) -> Result<Vec<Instruction>, String> {
  let mut instructions: Vec<Instruction> = Vec::new();

  // Get the single top-level Pair for the PROGRAM rule
  let mut program_pairs = pairs;
  let program_pair = program_pairs
    .next()
    .ok_or("Expected PROGRAM rule at top level")?;

  if program_pair.as_rule() != Rule::PROGRAM {
    return Err(format!(
      "Expected top-level rule PROGRAM, but got {:?}",
      program_pair.as_rule()
    ));
  }
  if program_pairs.next().is_some() {
    return Err("Expected only one top-level PROGRAM pair".to_string());
  }

  // Iterate over the inner pairs matched by the PROGRAM rule (these should be STMT rules)
  let statement_pairs = program_pair.into_inner();

  for stmt_pair in statement_pairs {
    println!("stmt_pair: {:?}", stmt_pair.as_str());
    // Each inner pair of PROGRAM should be a STMT rule based on your grammar PROGRAM = {(STMT)+}
    // If your PROGRAM rule allowed other things like EOL or WHITESPACE directly, you'd check here
    if stmt_pair.as_rule() != Rule::STMT {
      return Err(format!(
        "Expected STMT rule inside PROGRAM, but got {:?}",
        stmt_pair.as_rule()
      ));
    }

    // A STMT pair should have exactly one inner pair: either STMT_REG, STMT_IM, or STMT_ADDR
    let mut stmt_inner_pairs = stmt_pair.into_inner();
    let statement_type_pair = stmt_inner_pairs
      .next()
      .ok_or("Expected statement type rule inside STMT")?;

    if stmt_inner_pairs.next().is_some() {
      // This should not happen based on STMT = { STMT_REG | STMT_IM | STMT_ADDR }
      return Err(format!(
        "Unexpected multiple inner rules for STMT: {:?}",
        statement_type_pair.as_rule()
      ));
    }

    // Now, 'statement_type_pair' is the Pair for STMT_REG, STMT_IM, or STMT_ADDR
    // Since these rules are atomic, we get the full matched string from their span
    let statement_str = statement_type_pair.as_span().as_str().trim(); // Trim trailing EOL for easier splitting

    // --- Process the string based on the matched statement type ---
    let instruction = match statement_type_pair.as_rule() {
      Rule::STMT_REG => {
        // Format: REGISTER_KEYWORD WHITESPACE REGISTER WHITESPACE REGISTER EOL
        // Example string: "STO R1 R2"
        let parts: Vec<&str> = statement_str.split_whitespace().collect();
        if parts.len() != 3 {
          return Err(format!(
            "Invalid STMT_REG format string: Expected 3 parts, got '{}'",
            statement_str
          ));
        }
        let mnemonic = parts[0].to_string();
        let reg1 = parse_register(parts[1])?;
        let reg2 = parse_register(parts[2])?;
        Instruction {
          mnemonic,
          operand1: Some(Operand::Register(reg1)),
          operand2: Some(Operand::Register(reg2)),
        }
      }
      Rule::STMT_IM => {
        // Format: ("LOAD" | "CMP") WHITESPACE REGISTER WHITESPACE number EOL
        // Example string: "LOAD R10 10"
        let parts: Vec<&str> = statement_str.split_whitespace().collect();
        if parts.len() != 3 {
          return Err(format!(
            "Invalid STMT_IM format string: Expected 3 parts, got '{}'",
            statement_str
          ));
        }
        let mnemonic = parts[0].to_string();
        let reg = parse_register(parts[1])?;
        let immediate = parse_number(parts[2])?;
        Instruction {
          mnemonic,
          operand1: Some(Operand::Register(reg)),
          operand2: Some(Operand::Immediate(immediate)),
        }
      }
      Rule::STMT_ADDR => {
        // Format: ("JMP") WHITESPACE address EOL
        // Example string: "JMP 8" or "JMP 1024"
        let parts: Vec<&str> = statement_str.split_whitespace().collect();
        if parts.len() != 2 {
          return Err(format!(
            "Invalid STMT_ADDR format string: Expected 2 parts, got '{}'",
            statement_str
          ));
        }
        let mnemonic = parts[0].to_string();
        let address = parse_address(parts[1])?;
        Instruction {
          mnemonic,
          operand1: Some(Operand::Address(address)),
          operand2: None, // JMP usually has only one operand
        }
      }
      _ => {
        // This case should theoretically not be reached if STMT only alternates the above
        return Err(format!(
          "Unexpected rule type matched as statement type: {:?}",
          statement_type_pair.as_rule()
        ));
      }
    };
    instructions.push(instruction);
  }

  Ok(instructions)
}

fn main() {
  let fibonacci_program_no_comments_asm = r#"LOAD R0 1
LOAD R1 1
LOAD R5 5
LOAD R6 1
SUB R5 R6
LOAD R3 R0
ADD R0 R1
LOAD R1 R3
STYX R5 R1
JMP 8
    "#;

  let parse_result = AssemblerParser::parse(Rule::PROGRAM, fibonacci_program_no_comments_asm);

  match parse_result {
    Ok(pairs) => {
      println!("Parse successful. Processing tree...");
      // Process the parse tree
      match process_parse_tree(pairs) {
        Ok(instructions) => {
          println!("Successfully extracted instructions:");
          for instr in instructions {
            println!("{:?}", instr);
          }
        }
        Err(e) => {
          eprintln!("Error processing parse tree: {}", e);
        }
      }
    }
    Err(e) => {
      eprintln!("Parse error: {}", e);
    }
  }
  //println!("{:?}",AssemblerParser::parse(Rule::number, "255"));
  //println!("{:?}",AssemblerParser::parse(Rule::STMT_REG, "STO R1 R2\r\n"));
  //println!("{:?}",AssemblerParser::parse(Rule::STMT_REG, "STO R1 R2\r\n"));
  //println!("{:?}",AssemblerParser::parse(Rule::STMT, "LOAD R1 R2\n"));
  //println!("{:?}",AssemblerParser::parse(Rule::STMT, "LOAD R3 R2\n"));
  //println!("{:?}",AssemblerParser::parse(Rule::STMT, "LOAD R9 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::REGISTER, "R8"));
  //println!("{:?}", AssemblerParser::parse(Rule::REGISTER, "R10"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "STO R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "STO R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "LOAD R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "STYX R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT_IM, "LOAD R10 10\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT_IM, "LOAD R10 8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT_IM, "CMP R10 8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "CMP R10 8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "ADD R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT_ADDR, "JMP 1024\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "JMP 1024\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, fibonacci_program_no_comments_asm));
  //println!("{:?}", AssemblerParser::parse(Rule::STMT, "LOAD R10 R8\nSTO R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::PROGRAM, "LOAD R10 R8\nSTO R10 R8\n"));
  //println!("{:?}", AssemblerParser::parse(Rule::PROGRAM, fibonacci_program_no_comments_asm));

  //let path = Path::new("hello.bin");
  //let display = path.display();

  //// Open the path in read-only mode, returns `io::Result<File>`
  //let mut file = match File::open(&path) {
  //    Err(why) => panic!("couldn't open {}: {}", display, why),
  //    Ok(file) => file,
  //};

  //let mut cpu = CPU {
  //  registers: [0; 16],
  //  memory: [0; 4096],
  //  position_in_memory: 0,
  //  stack: [0; 16],
  //  stack_pointer: 0,
  //};

  //file.read(&mut cpu.memory).expect("Error reading file");

  //println!("{:?}", cpu.memory);
}

#[cfg(test)]
mod tests {
  // We need to bring the CPU struct into the scope of our tests module
  use super::*;

  #[test]
  fn subtract() {
    let mut cpu = CPU {
      registers: [0; 16],
      memory: [0; 4096],
      position_in_memory: 0x000, // Standard program start address
      stack: [0; 16],
      stack_pointer: 0,
    };

    let mut program: [u8; 6] = [
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
      assert_eq!(cpu.registers[i], 0, "V{} should be 0", i);
    }
  }

  #[test]
  // Test case where the conditional skip does NOT happen and a jump is executed
  fn test_program_no_skip_and_jump() {
    let mut cpu = CPU {
      registers: [0; 16],
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
      assert_eq!(cpu.registers[i], 0, "V{} should be 0", i);
    }
  }
}
