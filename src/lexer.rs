use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // keywords
    Struct, Effect, Const, Var, If, Else, While, Return,
    Print, Input, Perform, Void, I32, Mut,

    // symbols
    LBrace, RBrace, LParen, RParen, LBracket, RBracket,
    Comma, Semicolon, Colon, Arrow, Dot,
    Plus, Minus, Star, Slash, Percent,
    And, Or, Not, Eq, EqEq, Neq, Lt, Gt, Le, Ge,

    // literals / identifiers
    Ident(String),
    Number(i64),

    // end of file
    EOF,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer { input: source.chars().peekable() }
    }

    fn next_char(&mut self) -> Option<char> {
        self.input.next()
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_char(), Some(c) if c.is_whitespace()) {
            self.next_char();
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        let c = match self.next_char() {
            Some(ch) => ch,
            None => return Token::EOF,
        };

        match c {
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            ':' => Token::Colon,
            '.' => Token::Dot,
            '=' => Token::Eq,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            d if d.is_ascii_digit() => {
                let mut num = d.to_string();
                while matches!(self.peek_char(), Some(n) if n.is_ascii_digit()) {
                    num.push(self.next_char().unwrap());
                }
                Token::Number(num.parse().unwrap())
            }
            a if a.is_ascii_alphabetic() || a == '_' => {
                let mut ident = a.to_string();
                while matches!(self.peek_char(), Some(ch) if ch.is_ascii_alphanumeric() || *ch == '_') {
                    ident.push(self.next_char().unwrap());
                }
                match ident.as_str() {
                    "struct" => Token::Struct,
                    "effect" => Token::Effect,
                    "const" => Token::Const,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "return" => Token::Return,
                    "print" => Token::Print,
                    "input" => Token::Input,
                    "perform" => Token::Perform,
                    "i32" => Token::I32,
                    "void" => Token::Void,
                    _ => Token::Ident(ident),
                }
            }
            _ => Token::EOF,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            if tok == Token::EOF {
                tokens.push(Token::EOF);
                break;
            }
            tokens.push(tok);
        }
        tokens
    }
}