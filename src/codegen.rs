use std::arch::global_asm;
// src/codegen.rs
use std::collections::HashMap;

use crate::ast::*;
use crate::ir::{Instr, Func, ProgramIR};

pub struct Codegen;

impl Codegen {
    pub fn new() -> Self { Self }

    pub fn compile(&mut self, program: &Program) -> ProgramIR {
        // Compile top-level consts (ignored for now) and functions.
        // We’ll require a `main` function.
        let mut globals: HashMap<String, i32> = HashMap::new();
        for d in &program.decls {
            if let TopDecl::Const(c) = d {
                if let Expr::Number(n) = c.value {
                    globals.insert(c.name.clone(), n as i32);
                }
            }
        }

        let mut funcs = Vec::new();
        for d in &program.decls {
            match d {
                TopDecl::Func(f) => funcs.push(self.compile_func(f, &globals)),
                TopDecl::Const(_) => { /* could store in a global pool later */ }
                TopDecl::Struct(_) => { /* type-only, no code */ }
                TopDecl::Var(_) => { /* top-level vars unsupported in this MVP */ }
                TopDecl::Effect(_) => { /* placeholder */ }
            }
        }

        ProgramIR { funcs }
    }

    fn compile_func(&mut self, f: &FuncDef, globals: &HashMap<String, i32>) -> Func {
        // Local env: name -> slot
        let mut env = LocalEnv::default();

        // Allocate params first (left-to-right)
        for p in &f.params {
            env.alloc(&p.name);
        }

        let mut code = Vec::new();
        self.emit_block(&f.body, &mut env, globals, &mut code);

        // Ensure a Ret exists
        code.push(Instr::Ret);

        Func {
            name: f.name.clone(),
            code,
            n_locals: env.next,
            locals_dbg: env.reverse_names(),
        }
    }

    fn emit_block(&mut self, b: &Block, env: &mut LocalEnv, globals: &HashMap<String, i32>, code: &mut Vec<Instr>) {
        // Simple linear block
        for s in &b.stmts {
            self.emit_stmt(s, env, &globals, code);
        }
    }

    fn emit_stmt(&mut self, s: &Stmt, env: &mut LocalEnv, globals: &HashMap<String, i32>, code: &mut Vec<Instr>) {
        match s {
            Stmt::VarDecl(v) => {
                let idx = env.alloc(&v.name);
                if let Some(e) = &v.value {
                    self.emit_expr(e, env, globals, code);
                    code.push(Instr::Store(idx));
                } else {
                    // default 0
                    code.push(Instr::PushI32(0));
                    code.push(Instr::Store(idx));
                }
            }
            Stmt::ConstDecl(c) => {
                // Treat like immutable local in this MVP
                let idx = env.alloc(&c.name);
                self.emit_expr(&c.value, env, globals, code);
                code.push(Instr::Store(idx));
            }
            Stmt::Assign(a) => {
                // Minimal MVP: support only simple `name = expr;`
                let idx = env.lookup(&a.name).unwrap_or_else(|| {
                    panic!("assign to undeclared variable `{}`", a.name)
                });
                self.emit_expr(&a.value, env, globals, code);
                code.push(Instr::Store(idx));
            }
            Stmt::Expr(e) => {
                self.emit_expr(e, env, globals, code);
                code.push(Instr::Pop); // discard value of expr-stmt
            }
            Stmt::Return(opt) => {
                if let Some(e) = opt {
                    self.emit_expr(e, env, globals, code);
                }
                code.push(Instr::Ret);
            }
            Stmt::If(_) | Stmt::While(_) => {
                // Not yet (your parser accepts them; we’ll add control flow later)
                panic!("if/while not implemented in codegen MVP");
            }
        }
    }

    fn emit_expr(&mut self, e: &Expr, env: &mut LocalEnv, globals: &HashMap<String, i32>, code: &mut Vec<Instr>) {
        match e {
            Expr::Number(n) => code.push(Instr::PushI32(*n as i32)),
            Expr::Ident(name) => {
                if let Some(idx) = env.lookup(name) {
                    code.push(Instr::Load(idx))
                } else if let Some(value) = globals.get(name) {
                    code.push(Instr::PushI32(*value));
                } else {
                    panic!("use of undeclared variable `{}`", name);
                }
            }
            Expr::Builtin(b) => match b {
                Builtin::Print(arg) => {
                    self.emit_expr(arg, env, globals, code);
                    code.push(Instr::Print);
                    // Print consumes its argument, pushes nothing
                    // (so expr value is "unit"; caller often Pop's it if needed)
                }
                Builtin::Input => {
                    // MVP: just push 0; real impl could read from stdin later
                    code.push(Instr::PushI32(0));
                }
                Builtin::Perform(_, _) => {
                    panic!("perform not implemented in codegen MVP");
                }
            },

            // If you’ve already added Binary/Unary variants, handle them here.
            // For the MVP from Step 6, we only had simple literals/idents/print.
            Expr::Unary { .. } | Expr::Binary { .. } | Expr::Call { .. } => {
                panic!("complex expr not implemented in codegen MVP");
            }
        }
    }
}

#[derive(Default)]
struct LocalEnv {
    map: HashMap<String, usize>,
    names: Vec<String>,
    next: usize,
}

impl LocalEnv {
    fn alloc(&mut self, name: &str) -> usize {
        if let Some(&i) = self.map.get(name) {
            return i;
        }
        let idx = self.next;
        self.next += 1;
        self.map.insert(name.to_string(), idx);
        self.names.push(name.to_string());
        idx
    }
    fn lookup(&self, name: &str) -> Option<usize> {
        self.map.get(name).copied()
    }
    fn reverse_names(&self) -> Vec<String> {
        let mut v = vec!["".to_string(); self.next];
        for (n, &i) in &self.map {
            v[i] = n.clone();
        }
        v
    }
}
