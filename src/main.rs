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

mod samplegen;

fn main() -> Result<(), std::io::Error> {
    // Example source (fits your Step 6 features)
    
    /*let source = r#"
        struct Point {
            i32 x;
            i32 y;
        };

        const i32 n = 5;

        i32 main() {
            i32 x = 10;
            print(x);
            print(n);
            return 0;
        }
    "#;

    match compile_and_run(source) {
        Ok(code) => {
            println!("(exit code: {code})");
        }
        Err(e) => {
            println!("❌ {e}");
        }
    }
    */
    samplegen::emit_min_elf_hello("hello")?;
    println!("✅ ELF file generated");
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
