use crate::lexer::Token;
use crate::ast::*;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn next(&mut self) -> Token {
        let tok = self.peek().clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) {
        let got = self.next();
        if &got != expected {
            panic!("Expected {:?}, got {:?}", expected, got);
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut decls = Vec::new();
        while *self.peek() != Token::EOF {
            decls.push(self.parse_top_decl());
        }
        Program { decls }
    }

    fn parse_top_decl(&mut self) -> TopDecl {
        match self.peek() {
            Token::Struct => TopDecl::Struct(self.parse_struct_decl()),
            Token::Const => TopDecl::Const(self.parse_const_decl()),
            _ => panic!("unexpected token at top-level: {:?}", self.peek()),
        }
    }

    fn parse_struct_decl(&mut self) -> StructDecl {
        self.expect(&Token::Struct);
        let name = match self.next() {
            Token::Ident(id) => id,
            t => panic!("expected struct name, got {:?}", t),
        };
        self.expect(&Token::LBrace);

        let mut fields = Vec::new();
        while *self.peek() != Token::RBrace {
            fields.push(self.parse_field());
        }
        self.expect(&Token::RBrace);
        self.expect(&Token::Semicolon);
        StructDecl { name, fields }
    }

    fn parse_field(&mut self) -> Field {
        let ty = self.parse_type();
        let name = match self.next() {
            Token::Ident(id) => id,
            t => panic!("expected field name, got {:?}", t),
        };
        self.expect(&Token::Semicolon);
        Field { ty, name }
    }

    fn parse_type(&mut self) -> Type {
        match self.next() {
            Token::I32 => Type { name: "i32".to_string() },
            Token::Ident(id) => Type { name: id },
            t => panic!("expected type, got {:?}", t),
        }
    }

    fn parse_const_decl(&mut self) -> ConstDecl {
        self.expect(&Token::Const);
        let ty = self.parse_type();
        let name = match self.next() {
            Token::Ident(id) => id,
            t => panic!("expected identifier after type, got {:?}", t),
        };
        self.expect(&Token::Eq);
        let value = self.parse_expr();
        self.expect(&Token::Semicolon);
        ConstDecl { ty, name, value }
    }

    fn parse_expr(&mut self) -> Expr {
        match self.next() {
            Token::Number(n) => Expr::Number(n),
            Token::Ident(id) => Expr::Ident(id),
            Token::Print => {
                self.expect(&Token::LParen);
                let arg = self.parse_expr();
                self.expect(&Token::RParen);
                Expr::Builtin(Builtin::Print(Box::new(arg)))
            }
            t => panic!("unexpected token in expr: {:?}", t),
        }
    }
}
