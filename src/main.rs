use std::env;
use std::fs;

mod brainfuck;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => help(args[0].as_str()),
        2 => run(read_file(&args[1]).as_str(), None),
        3 => run(
            read_file(&args[1]).as_str(),
            Some(args[2].as_str().parse::<usize>().unwrap()),
        ),
        _ => help("./brainfuck"),
    }
}

fn read_file(file_name: &str) -> String {
    fs::read_to_string(file_name).unwrap()
}

fn run(program: &str, memory_capacity: Option<usize>) {
    if let Some(program) = brainfuck::Program::parse(program) {
        let memory_capacity = memory_capacity.unwrap_or(1048576);
        let mut memory = vec![0u8; memory_capacity];
        program.run(&mut memory);
    }
}

fn help(program_line: &str) {
    panic!("Usage:\n\t{} <program.bf> [memory-size]\n\nMemory size in bytes. Defaults to 1MiB (1048576 bytes)", program_line)
}
