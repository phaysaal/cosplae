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

    pub fn peek(&self) -> &Token {
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

    // ---- program ----
    pub fn parse_program(&mut self) -> Program {
        let mut decls = Vec::new();
        while *self.peek() != Token::EOF {
            decls.push(self.parse_top_decl());
        }
        Program { decls }
    }

    // ---- top_decl ----
    fn parse_top_decl(&mut self) -> TopDecl {
        match self.peek() {
            Token::Struct => TopDecl::Struct(self.parse_struct_decl()),
            Token::Const  => TopDecl::Const(self.parse_const_decl()),
            Token::I32 | Token::Ident(_) => {
                // Could be a function definition
                let ty = self.parse_type();
                let name = match self.next() {
                    Token::Ident(id) => id,
                    t => panic!("Expected function name, got {:?}", t),
                };
                self.expect(&Token::LParen);
                let params = self.parse_params();
                self.expect(&Token::RParen);
                let body = self.parse_block();
                TopDecl::Func(FuncDef { ret_type: ty, name, params, body })
            }
            _ => panic!("Unexpected token in top_decl: {:?}", self.peek()),
        }
    }

    // ---- struct_decl ----
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

    // ---- parameters ----
    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while let Token::I32 | Token::Ident(_) = self.peek() {
            let ty = self.parse_type();
            let name = match self.next() {
                Token::Ident(id) => id,
                t => panic!("expected param name, got {:?}", t),
            };
            params.push(Param { ty, name });
            if *self.peek() == Token::Comma {
                self.next();
            } else {
                break;
            }
        }
        params
    }

    // ---- block ----
    fn parse_block(&mut self) -> Block {
        self.expect(&Token::LBrace);
        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace {
            stmts.push(self.parse_stmt());
        }
        self.expect(&Token::RBrace);
        Block { stmts }
    }

    // ---- statement ----
    fn parse_stmt(&mut self) -> Stmt {
        match self.peek() {
            Token::Const => Stmt::ConstDecl(self.parse_const_decl()),
            Token::Return => Stmt::Return(self.parse_return_stmt()),
            Token::I32 | Token::Ident(_) => {
                // Could be var_decl or expr
                // Look ahead to decide
                let pos = self.pos;
                let ty = self.parse_type();
                if let Token::Ident(id) = self.next() {
                    if *self.peek() == Token::Eq {
                        self.next();
                        let expr = self.parse_expr();
                        self.expect(&Token::Semicolon);
                        Stmt::VarDecl(VarDecl { ty, name: id, value: Some(expr) })
                    } else if *self.peek() == Token::Semicolon {
                        self.next();
                        Stmt::VarDecl(VarDecl { ty, name: id, value: None })
                    } else {
                        // restore position → expression statement
                        self.pos = pos;
                        let e = self.parse_expr();
                        self.expect(&Token::Semicolon);
                        Stmt::Expr(e)
                    }
                } else {
                    panic!("Expected identifier after type or expression");
                }
            }
            _ => {
                let e = self.parse_expr();
                self.expect(&Token::Semicolon);
                Stmt::Expr(e)
            }
        }
    }

    fn parse_return_stmt(&mut self) -> Option<Expr> {
        self.expect(&Token::Return);
        let expr = if *self.peek() == Token::Semicolon {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(&Token::Semicolon);
        expr
    }


    // ---- const_decl ----
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

    // ---- expr ----
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
