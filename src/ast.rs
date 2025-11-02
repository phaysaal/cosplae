#[allow(dead_code)]
#[derive(Debug)]
pub struct Program {
    pub decls: Vec<TopDecl>,
}

#[derive(Debug)]
pub enum TopDecl {
    Struct(StructDecl),
    Const(ConstDecl),
    Func(FuncDef),
    Var(VarDecl),          // stub for future top-level vars
    Effect(EffectDecl),    // stub for effect declarations
}

#[derive(Debug)]
pub struct EffectDecl {
    pub name: String,
    pub params: Vec<Type>,
    pub ret: Option<Type>,
}

#[derive(Debug)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug)]
pub struct Field {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug)]
pub struct ConstDecl {
    pub ty: Type,
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub struct FuncDef {
    pub ret_type: Type,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Block,
}

#[derive(Debug)]
pub struct Param {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    VarDecl(VarDecl),
    ConstDecl(ConstDecl),
    Assign(Assign),
    Expr(Expr),
    Return(Option<Expr>),
    If(IfStmt),        // stub
    While(WhileStmt),  // stub
}

#[derive(Debug)]
pub struct IfStmt {
    pub cond: Expr,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

#[derive(Debug)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Block,
}


#[derive(Debug)]
pub struct VarDecl {
    pub ty: Type,
    pub name: String,
    pub value: Option<Expr>,
}

#[derive(Debug)]
pub struct Assign {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug)]
pub struct Type {
    pub name: String,
}

#[derive(Debug)]
pub enum Expr {
    Number(i64),
    Ident(String),
    Builtin(Builtin),
    Unary { op: String, expr: Box<Expr> },
    Binary { op: String, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}


#[derive(Debug)]
pub enum Builtin {
    Print(Box<Expr>),
    Input,
    Perform(String, Vec<Expr>),
}

