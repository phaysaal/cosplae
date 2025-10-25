#[derive(Debug)]
pub struct Program {
    pub decls: Vec<TopDecl>,
}

#[derive(Debug)]
pub enum TopDecl {
    Struct(StructDecl),
    Const(ConstDecl),
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
pub struct Type {
    pub name: String,
}

#[derive(Debug)]
pub enum Expr {
    Number(i64),
    Ident(String),
    Builtin(Builtin),
}

#[derive(Debug)]
pub enum Builtin {
    Print(Box<Expr>),
}
