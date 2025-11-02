// src/ir.rs
#[derive(Debug, Clone)]
pub enum Instr {
    // stack ops
    PushI32(i32),
    Pop,

    // locals
    Load(usize),   // push locals[idx]
    Store(usize),  // pop -> locals[idx]

    // arithmetic
    Add, Sub, Mul, Div,

    // builtins
    Print,         // pop & print as i32

    // control/return
    Ret,           // pop as function return (or 0 if stack empty)
}

// One function's code + its local layout
#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub code: Vec<Instr>,
    pub n_locals: usize,
    // optional: map variable index → name for debugging
    pub locals_dbg: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProgramIR {
    pub funcs: Vec<Func>, // index 0 must be "main"
}

impl ProgramIR {
    pub fn main_index(&self) -> Option<usize> {
        self.funcs.iter().position(|f| f.name == "main")
    }
}
