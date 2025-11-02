// src/vm.rs
use crate::ir::{Instr, ProgramIR};

pub struct VM;

impl VM {
    pub fn run(prog: &ProgramIR) -> i32 {
        let main_idx = prog.main_index().expect("no `main` function found");
        let main = &prog.funcs[main_idx];

        let mut stack: Vec<i32> = Vec::new();
        let mut locals: Vec<i32> = vec![0; main.n_locals];

        let mut ip: usize = 0; // instruction pointer

        while ip < main.code.len() {
            match &main.code[ip] {
                Instr::PushI32(n) => stack.push(*n),
                Instr::Pop => { stack.pop(); }

                Instr::Load(i) => stack.push(locals[*i]),
                Instr::Store(i) => {
                    let v = stack.pop().expect("stack underflow on Store");
                    locals[*i] = v;
                }

                Instr::Add => bin(&mut stack, |a,b| a+b),
                Instr::Sub => bin(&mut stack, |a,b| a-b),
                Instr::Mul => bin(&mut stack, |a,b| a*b),
                Instr::Div => bin(&mut stack, |a,b| a/b),

                Instr::Print => {
                    let v = stack.pop().expect("stack underflow on Print");
                    println!("{v}");
                }

                Instr::Ret => {
                    return stack.pop().unwrap_or(0);
                }
            }
            ip += 1;
        }

        // In case no explicit Ret got hit (we emit one anyway)
        0
    }
}

fn bin(stack: &mut Vec<i32>, f: impl Fn(i32, i32) -> i32) {
    let b = stack.pop().expect("stack underflow (rhs)");
    let a = stack.pop().expect("stack underflow (lhs)");
    stack.push(f(a, b));
}
