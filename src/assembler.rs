mod parser;

use self::parser::{Instruction, process_parse_tree};

use self::parser::AssemblerParser;
use self::parser::Operand;
use self::parser::Rule;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
fn resolve_label(mut instructions: Vec<Instruction>) -> Vec<Instruction> {
  let mut labels: HashMap<String, usize> = HashMap::new();

  for instr in &instructions {
    // Must be a label
    if instr.mnemonic.ends_with(":") {
      let label = instr.mnemonic.trim_end_matches(":");
      labels.insert(label.to_string(), instr.location.into());
    }
  }

  for instr in instructions.iter_mut() {
    match instr {
      Instruction {
        mnemonic,
        location: _,
        operand1,
        operand2,
      } if mnemonic == "JMP" => match (operand1, operand2) {
        (Some(Operand::Label(l1)), _) => {
          instr.operand1 = Some(Operand::Address(labels[l1] as u16));
        }
        (_, _) => {
          eprintln!("Error: Invalid SUB instruction");
          panic!();
        }
      },

      Instruction {
        mnemonic,
        location: _,
        operand1,
        operand2,
      } => {}
    }
  }

  instructions
}

fn assemble(instructions: Vec<Instruction>) -> [u8; 4096] {
  println!("Successfully extracted instructions:");
  let mut mem = [0u8; 4096];
  let mut instr_ctr: usize = 0;
  for instr in instructions {
    println!("{instr:?}");
    match instr {
      Instruction {
        ref mnemonic,
        location: _,
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
        location: _,
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
        location: _,
        operand1,
        operand2,
      } if mnemonic == "ADD" => match (operand1, operand2) {
        (Some(Operand::Register(r1)), Some(Operand::Register(r2))) => {
          mem[instr_ctr] = 0x80 | r1;
          mem[instr_ctr + 1] = (r2 << 4) | 0x4;
        }
        (_, _) => {
          eprintln!("Error: Invalid ADD instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location: _,
        operand1,
        operand2,
      } if mnemonic == "SUB" => match (operand1, operand2) {
        (Some(Operand::Register(r1)), Some(Operand::Register(r2))) => {
          mem[instr_ctr] = 0x80 | r1;
          mem[instr_ctr + 1] = (r2 << 4) | 0x5;
        }
        (_, _) => {
          eprintln!("Error: Invalid SUB instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location: _,
        operand1,
        operand2,
      } if mnemonic == "CMP" => match (operand1, operand2) {
        (Some(Operand::Register(r1)), Some(Operand::Immediate(i2))) => {
          mem[instr_ctr] = 0x30 | r1;
          mem[instr_ctr + 1] = i2 << 4;
        }
        (_, _) => {
          eprintln!("Error: Invalid CMP instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location: _,
        operand1,
        operand2,
      } if mnemonic == "JMP" => match (operand1, operand2) {
        (Some(Operand::Address(r1)), _) => {
          mem[instr_ctr] = 0x10 | ((r1 >> 8) & 0x000F) as u8;
          mem[instr_ctr + 1] = (r1 & 0x00FF) as u8;
        }
        (_, _) => {
          eprintln!("Error: Invalid JMP instruction");
          panic!();
        }
      },
      Instruction {
        ref mnemonic,
        location: _,
        operand1: _,
        operand2: _,
      } if mnemonic.ends_with(":") => {
        // Labels are saved in the instruction list, for each instr. we inc. the instr_ctr but
        // labels are fake instructions that are not parsed into bytes so we need to subtract 2
        instr_ctr -= 2;
      }
      Instruction {
        mnemonic: ref _ign,
        location: ref _ign2,
        operand1: ref _ign3,
        operand2: ref _ign4,
      } => {
        eprintln!("Error: Unknown instruction instr.: {instr:?}");
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
  //  let fibonacci_program_no_comments_asm = r#"LOAD R5 1
  //LOAD R6 1
  //ADD R5 R6
  //    "#;

  let parse_result = AssemblerParser::parse(Rule::PROGRAM, fibonacci_program_no_comments_asm);

  match parse_result {
    Ok(pairs) => {
      println!("Parse successful. Processing tree...");
      // Process the parse tree
      match process_parse_tree(pairs) {
        Ok(mut instructions) => {
          println!("{:?}", instructions);
          instructions = resolve_label(instructions);
          println!("{:?}", instructions);
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
        mnemonic: "NOP".to_string(),
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

  #[test]
  fn test_assemble_add() {
    let instructions = vec![Instruction {
      mnemonic: "ADD".to_string(),
      location: 0,
      operand1: Some(Operand::Register(6)),
      operand2: Some(Operand::Register(2)),
    }];

    let expected_bytes: [u8; 2] = [0x86, 0x24];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..2], &expected_bytes);

    assert_eq!(&memory[2..], &zeroed_memory()[2..]);
  }

  #[test]
  fn test_assemble_sub() {
    let instructions = vec![Instruction {
      mnemonic: "SUB".to_string(),
      location: 0,
      operand1: Some(Operand::Register(6)),
      operand2: Some(Operand::Register(2)),
    }];

    let expected_bytes: [u8; 2] = [0x86, 0x25];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..2], &expected_bytes);

    assert_eq!(&memory[2..], &zeroed_memory()[2..]);
  }

  #[test]
  fn test_fib() {
    let instructions = vec![
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 0,
        operand1: Some(Operand::Register(0)),
        operand2: Some(Operand::Immediate(1)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 1,
        operand1: Some(Operand::Register(1)),
        operand2: Some(Operand::Immediate(1)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 2,
        operand1: Some(Operand::Register(5)),
        operand2: Some(Operand::Immediate(5)),
      },
      Instruction {
        mnemonic: "LOAD".to_string(),
        location: 3,
        operand1: Some(Operand::Register(6)),
        operand2: Some(Operand::Immediate(1)),
      },
      Instruction {
        mnemonic: "SUB".to_string(),
        location: 4,
        operand1: Some(Operand::Register(5)),
        operand2: Some(Operand::Register(6)),
      },
      // loop label is ignored in instruction structure unless label support is added
      Instruction {
        mnemonic: "STYX".to_string(),
        location: 5,
        operand1: Some(Operand::Register(3)),
        operand2: Some(Operand::Register(0)),
      },
      Instruction {
        mnemonic: "ADD".to_string(),
        location: 6,
        operand1: Some(Operand::Register(0)),
        operand2: Some(Operand::Register(1)),
      },
      Instruction {
        mnemonic: "STYX".to_string(),
        location: 7,
        operand1: Some(Operand::Register(1)),
        operand2: Some(Operand::Register(3)),
      },
      Instruction {
        mnemonic: "CMP".to_string(),
        location: 8,
        operand1: Some(Operand::Register(5)),
        operand2: Some(Operand::Immediate(0)),
      },
      Instruction {
        mnemonic: "JMP".to_string(),
        location: 9,
        operand1: Some(Operand::Address(8)),
        operand2: None,
      },
    ];
    const LEN: usize = 22;
    let expected_bytes: [u8; LEN] = [
      // Main program (starts at 0x200)
      0x60, 0x01, // 0x002: LD V0, 0x01
      0x61, 0x01, // 0x004: LD V1, 0x01
      0x65, 0x05, // 0x006: LD V5, 0x05
      0x66, 0x01, // 0x008: LD V6, 0x01
      0x85, 0x65, // 0x00A: V5 <- V5 - V6 // Decrement counter
      0x83, 0x00, //      : V3 <- V0 (STYX V3 V0)      // Save f_n+1
      0x80, 0x14, // 0x00C: ADD V0, V1
      0x81, 0x30, //      : V1 <- V3 (STYX V3 V1)
      0x35, 0x00, //      : V5 == 0;
      0x10, 0x08, // JMP 0x008
      0x00, 0x00,
    ];

    let memory = assemble(instructions);

    assert_eq!(&memory[0..LEN], &expected_bytes);

    assert_eq!(&memory[LEN..], &zeroed_memory()[LEN..]);
  }
}
