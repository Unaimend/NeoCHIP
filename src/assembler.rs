mod parser;

use self::parser::{Instruction, process_parse_tree};

use self::parser::AssemblerParser;
use self::parser::Operand;
use self::parser::Rule;

use pest::Parser;
use pest_derive::Parser;

fn assemble(instructions: Vec<Instruction>) -> [u8; 4096] {
  println!("Successfully extracted instructions:");
  let mut mem = [0u8; 4096];
  let mut instr_ctr: usize = 0;
  for instr in instructions {
    println!("{instr:?}");
    match instr {
      Instruction {
        ref mnemonic,
        location,
        operand1,
        operand2,
      } if mnemonic == "LOAD" => match (operand1, operand2) {
        (Some(Operand::Register(r1)), Some(Operand::Immediate(i2))) => {
          mem[instr_ctr] = 0x60 | r1;
          mem[instr_ctr + 1] = i2;
        }
        (_, _) => {
          eprintln!("Error: Invalid LOAD instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location,
        operand1,
        operand2,
      } if mnemonic == "STYX" => match (operand1, operand2) {
        (Some(Operand::Register(r1)), Some(Operand::Register(r2))) => {
          mem[instr_ctr] = 0x80 | r1;
          mem[instr_ctr + 1] = (r2 << 4);
        }
        (_, _) => {
          eprintln!("Error: Invalid LOAD instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location,
        operand1,
        operand2,
      } => {
        eprintln!("Error: Unknown instruction mnemonic: {mnemonic:?}");
      }
    }
    instr_ctr += 2;
  }
  mem
}

fn main() {
  let fibonacci_program_no_comments_asm = r#"LOAD R0 1
LOAD R1 1
LOAD R5 5
LOAD R6 1
SUB R5 R6
loop:
STYX R3 R0
ADD R0 R1
STYX R1 R3
CMP R5 0
JMP loop
    "#;

  let parse_result = AssemblerParser::parse(Rule::PROGRAM, fibonacci_program_no_comments_asm);

  match parse_result {
    Ok(pairs) => {
      println!("Parse successful. Processing tree...");
      // Process the parse tree
      match process_parse_tree(pairs) {
        Ok(instructions) => {
          let binary_code = assemble(instructions);
          for i in (0..20).step_by(2) {
            println!("{:02X} {:02X}", binary_code[i], binary_code[i + 1]);
          }
        }
        Err(e) => {
          eprintln!("Error processing parse tree: {e}");
        }
      }
    }
    Err(e) => {
      eprintln!("Parse error: {e}");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn zeroed_memory() -> [u8; 4096] {
    [0u8; 4096]
  }

  #[test]
  fn test_assemble_single_load_register_immediate() {
    let instructions = vec![Instruction {
      mnemonic: "LOAD".to_string(),
      location: 0,
      operand1: Some(Operand::Register(0)),
      operand2: Some(Operand::Immediate(1)),
    }];

    let expected_bytes: [u8; 2] = [0x60, 0x01];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..2], &expected_bytes);

    assert_eq!(&memory[2..], &zeroed_memory()[2..]);
  }

  #[test]
  fn test_assemble_multiple_load_register_immediate() {
    let instructions = vec![
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 0,
        operand1: Some(Operand::Register(1)),
        operand2: Some(Operand::Immediate(10)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 2,
        operand1: Some(Operand::Register(5)),
        operand2: Some(Operand::Immediate(255)),
      },
    ];

    let expected_bytes: [u8; 4] = [0x61, 0x0A, 0x65, 0xFF];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..4], &expected_bytes);
    assert_eq!(&memory[4..], &zeroed_memory()[4..]);
  }

  #[test]
  fn test_assemble_load_register_immediate_edge_values() {
    let instructions = vec![
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 0,
        operand1: Some(Operand::Register(0)),
        operand2: Some(Operand::Immediate(0)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 2,
        operand1: Some(Operand::Register(15)),
        operand2: Some(Operand::Immediate(255)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 4,
        operand1: Some(Operand::Register(7)),
        operand2: Some(Operand::Immediate(1)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 6,
        operand1: Some(Operand::Register(8)),
        operand2: Some(Operand::Immediate(254)),
      },
    ];

    let expected_bytes: [u8; 8] = [0x60, 0x00, 0x6F, 0xFF, 0x67, 0x01, 0x68, 0xFE];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..8], &expected_bytes);
    assert_eq!(&memory[8..], &zeroed_memory()[8..]);
  }

  #[test]
  fn test_assemble_unhandled_instruction() {
    let instructions = vec![
      Instruction {
        mnemonic: "ADD".to_string(),
        location: 0,
        operand1: Some(Operand::Register(0)),
        operand2: Some(Operand::Register(1)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 2,
        operand1: Some(Operand::Register(2)),
        operand2: Some(Operand::Immediate(5)),
      },
    ];

    let expected_bytes: [u8; 4] = [0x00, 0x00, 0x62, 0x05];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..4], &expected_bytes);
    assert_eq!(&memory[4..], &zeroed_memory()[4..]);
  }
}
