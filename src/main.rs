mod lexer;
mod parser;
mod ast;

use lexer::Lexer;
use parser::Parser;

fn main() {
    let source = r#"
        struct Point {
            i32 x;
            i32 y;
        };

        const i32 n = 5;
        print(n);
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    println!("TOKENS: {tokens:#?}");

    let mut parser = Parser::new(tokens);
    let ast = parser.parse_program();
    println!("AST: {ast:#?}");
}
