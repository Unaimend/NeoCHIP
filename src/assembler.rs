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
  let instr_ctr: usize = 0;
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
        (_, _) => {}
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
LOAD R3 R0
ADD R0 R1
LOAD R1 R3
STYX R5 R1
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
          for i in 0..50 {
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
