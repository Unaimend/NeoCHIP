mod assembler;
mod emulator;
mod parser;

use parser::{AssemblerParser, Rule, process_parse_tree};
use pest::Parser;

use assembler::{assemble, resolve_label};

use emulator::CPU;

use std::fs;

fn main() {
  let code = fs::read_to_string("examples/fibonacci.asm").unwrap();
  let parse_result = AssemblerParser::parse(Rule::PROGRAM, &code);

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

          let mut cpu = CPU {
            registers: [0; 16],
            register_i: 0,
            memory: [0; 4096],
            position_in_memory: 0x000, // Standard program start address
            stack: [0; 16],
            stack_pointer: 0,
          };

          cpu.memory[0..4096].copy_from_slice(&binary_code);
          cpu.run();
          println!("{:?}", cpu.registers);
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
