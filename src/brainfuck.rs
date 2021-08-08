use std::io::Read;
use std::io;
use itertools::*;

/*
// Usage:
if let Some(program) = brainfuck::Program::parse(program) {

    let mut memory = vec![0u8; 1048576];
    program.run(&mut memory);
}
*/


/// Increments/Decrements are parameterized with usize to be able to use the same data structure for optimization (see Self::optimize method).
#[derive(PartialEq, Clone, Copy, Debug)]
enum OpCode {
    /// ++ptr
    IncrementPointer,
    /// --ptr
    DecrementPointer,
    /// ++*ptr
    IncrementValue,
    /// --*ptr
    DecrementValue,
    /// putchar(*ptr)
    PutChar,
    /// *ptr = getchar()
    GetChar,
    /// while (*ptr) {
    LoopHead,
    /// }
    LoopEnd,
}

impl OpCode {
    fn parse(c: char) -> Option<OpCode> {
        match c {
            '>' => Some(OpCode::IncrementPointer),
            '<' => Some(OpCode::DecrementPointer),
            '+' => Some(OpCode::IncrementValue),
            '-' => Some(OpCode::DecrementValue),
            '.' => Some(OpCode::PutChar),
            ',' => Some(OpCode::GetChar),
            '[' => Some(OpCode::LoopHead),
            ']' => Some(OpCode::LoopEnd),
            _ => None,
        }
    }

    fn create_optimized_instruction(&self, n: usize) -> Instruction {
        assert!(n > 1);
        assert!(self.is_run_length_optimizable());

        match self {
            OpCode::DecrementPointer => Instruction::DecrementPointer(n),
            OpCode::IncrementPointer => Instruction::IncrementPointer(n),
            OpCode::DecrementValue => Instruction::DecrementValue(n),
            OpCode::IncrementValue => Instruction::IncrementValue(n),
            _ => panic!("Cannot optimize non-incrementing OpCode with more than 1 repetition"),
        }
    }

    fn as_instruction(&self) -> Instruction {
        match self {
            OpCode::DecrementPointer => Instruction::DecrementPointer(1),
            OpCode::IncrementPointer => Instruction::IncrementPointer(1),
            OpCode::DecrementValue => Instruction::DecrementValue(1),
            OpCode::IncrementValue => Instruction::IncrementValue(1),
            OpCode::PutChar => Instruction::PutChar,
            OpCode::GetChar => Instruction::GetChar,
            OpCode::LoopHead => Instruction::LoopHead(usize::MAX),
            OpCode::LoopEnd => Instruction::LoopEnd(usize::MAX),
        }
    }

    fn is_run_length_optimizable(&self) -> bool {
        matches!(
            self,
            OpCode::DecrementPointer
                | OpCode::IncrementPointer
                | OpCode::DecrementValue
                | OpCode::IncrementValue
        )
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Instruction {
    IncrementPointer(usize),
    DecrementPointer(usize),
    IncrementValue(usize),
    DecrementValue(usize),
    PutChar,
    GetChar,
    LoopHead(usize /* pointer to end instruction */),
    LoopEnd(usize /* pointer to head instruction */),
}

pub struct Program {
    instructions: Vec<Instruction>,
}

impl Program {
    pub fn parse(code: &str) -> Option<Program> {
        let op_codes: Vec<OpCode> = code.chars().filter_map(OpCode::parse).collect();

        if !Self::check(&op_codes) {
            return None;
        }

        let instructions = Self::bind(&op_codes);

        Some(Program { instructions })
    }

    fn check(op_codes: &[OpCode]) -> bool {
        Self::has_balanced_brackets(&op_codes)
    }

    fn has_balanced_brackets(op_codes: &[OpCode]) -> bool {
        let mut unclosed_loop_heads: isize = 0;

        for c in op_codes {
            match c {
                OpCode::LoopHead => unclosed_loop_heads += 1,
                OpCode::LoopEnd => {
                    unclosed_loop_heads -= 1;
                    if unclosed_loop_heads < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }

        unclosed_loop_heads == 0
    }

    fn bind(op_codes: &[OpCode]) -> Vec<Instruction> {
        // In the bind step, we don't only bind the loop heads/ends, we also compress the OpCodes by optimizing them:
        // For interpreting brainfuck, we can pull off a simple optimization:
        // Occurrences in the form of "++++" can be compressed into a single instruction (that's why we have the usize in the Instruction enum)
        // It essentially boils down to run-length-encoding of increment/decrement instructions

        let optimized_instructions: Vec<Instruction> = op_codes
            .iter()
            .group_by(|c| *c)
            .into_iter()
            .flat_map(|(&code, group)| match group.count() {
                1 => vec![code.as_instruction()],
                n => {
                    if code.is_run_length_optimizable() {
                        vec![code.create_optimized_instruction(n)]
                    } else {
                        vec![code.as_instruction(); n]
                    }
                }
            })
            .collect();

        let mut loop_head_address_stack = Vec::<usize>::new();

        let mut bound_instructions = optimized_instructions.to_vec();
        for (current_index, instruction) in optimized_instructions.iter().enumerate() {
            match instruction {
                Instruction::LoopHead(_) => loop_head_address_stack.push(current_index),
                Instruction::LoopEnd(_) => {
                    let corresponding_start_index = loop_head_address_stack.pop().unwrap();

                    // Set loop start address to the last loop head
                    bound_instructions[current_index] = Instruction::LoopEnd(corresponding_start_index);

                    // Set the loop end address of the start element to this address
                    bound_instructions[corresponding_start_index] = Instruction::LoopHead(current_index);
                }
                _ => {}
            }
        }

        assert!(loop_head_address_stack.is_empty());

/*
        println!(
            "optimized_instructions ({:?} -> {:?})",
            op_codes.len(),
            bound_instructions.len()
        );
*/

        bound_instructions
    }

    pub fn run(&self, memory: &mut [u8]) {
        let stdin = io::stdin();
        let mut stdin_bytes = stdin.lock().bytes();

        let mut instruction_pointer: isize = 0;
        let mut data_pointer: usize = 0;
        while 0 <= instruction_pointer && instruction_pointer < self.instructions.len() as isize {
            // casting to isize :/

            let current_instruction = self.instructions[instruction_pointer as usize];

            match current_instruction {
                Instruction::IncrementPointer(n) => {
                    data_pointer += n;
                    Self::panic_if_overflow(data_pointer, memory);

                    instruction_pointer += 1;
                }
                Instruction::DecrementPointer(n) => {
                    // TODO: This is ugly, there must be a better way
                    let next_value = (data_pointer as isize) - (n as isize);
                    Self::panic_if_underflow(next_value);
                    data_pointer = next_value as usize;

                    instruction_pointer += 1;
                }
                Instruction::IncrementValue(n) => {
                    memory[data_pointer] = ((memory[data_pointer] as usize) + n) as u8;

                    instruction_pointer += 1;
                }
                Instruction::DecrementValue(n) => {
                    memory[data_pointer] = ((memory[data_pointer] as usize) - n) as u8;

                    instruction_pointer += 1;
                }
                Instruction::LoopHead(loop_end_address) => {
                    if memory[data_pointer] == 0 {
                        instruction_pointer = (loop_end_address as isize) + 1;
                    } else {
                        instruction_pointer += 1;
                    }
                }
                Instruction::LoopEnd(loop_start_address) => {

                    if memory[data_pointer] == 0 {
                        instruction_pointer += 1;
                    } else {
                        instruction_pointer = loop_start_address as isize;
                    }
                }
                Instruction::PutChar => {
                    print!("{}", memory[data_pointer] as char);

                    instruction_pointer += 1;
                },
                Instruction::GetChar => {
                    let input = stdin_bytes.next();
                    if input.is_none() {
                        continue;
                    }

                    memory[data_pointer] = input.unwrap().unwrap();

                    instruction_pointer += 1;
                },
            }

        }
    }

    fn panic_if_overflow(data_pointer: usize, memory: &[u8]) {
        if data_pointer >= memory.len() {
            panic!("data pointer overflow");
        }
    }
    fn panic_if_underflow(data_pointer: isize) {
        if data_pointer < 0 {
            panic!("data pointer underflow");
        }
    }
}
