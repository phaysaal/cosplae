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

// mod samplegen;
mod elfgen;
use elfgen::ELFBuilder;

fn main() -> Result<(), std::io::Error> {
    // Example source (fits your Step 6 features)
    
    /* let source = r#"
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
    } */
    /* 
    samplegen::emit_min_elf_hello("hello")?;
    println!("✅ ELF file generated"); */
    

    let mut elf = ELFBuilder::new();

    // Minimal "write(1, msg, len); exit(0)" machine code
    let code: [u8; 42] = [
        0x48,0xC7,0xC0,0x01,0x00,0x00,0x00,   // mov rax,1
        0x48,0xC7,0xC7,0x01,0x00,0x00,0x00,   // mov rdi,1
        0x48,0x8D,0x35,0x0E,0x00,0x00,0x00,   // lea rsi,[rip+0xe]
        0x48,0xC7,0xC2,0x06,0x00,0x00,0x00,   // mov rdx,6
        0x0F,0x05,                             // syscall
        0x48,0xC7,0xC0,0x3C,0x00,0x00,0x00,   // mov rax,60
        0x48,0x31,0xFF,                       // xor rdi,rdi
        0x0F,0x05,                             // syscall
    ];

    elf.append_code(&code);
    elf.append_str("Hello\n");
    elf.emit("hello")?;
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
