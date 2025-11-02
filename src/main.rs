// src/main.rs
mod lexer;
mod parser;
mod ast;
mod ir;
mod codegen;
mod vm;

use lexer::Lexer;
use parser::Parser;
use codegen::Codegen;
use std::fs;
use std::env;

fn main() -> Result<(), std::io::Error> {
    // Get source file path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <source_file>", args[0]);
        std::process::exit(1);
    }

    let source_path = &args[1];
    let source = fs::read_to_string(source_path)?;

    match compile_and_run(&source) {
        Ok(code) => {
            println!("✅ Program compiled and executed successfully");
            println!("(exit code: {code})");
        }
        Err(e) => {
            eprintln!("❌ {e}");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn compile_and_run(source: &str) -> Result<i32, String> {
    // 1) Lex + parse
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let ast = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parser.parse_program()))
        .map_err(|_| "Parsing failed due to syntax error.".to_string())?;

    // 2) Codegen
    let mut cg = Codegen::new();
    let ir = cg.compile(&ast);

    // 3) Run VM
    let exit = vm::VM::run(&ir);

    Ok(exit)
}
