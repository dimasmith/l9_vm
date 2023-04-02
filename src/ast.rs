//! Abstract syntax tree for l9 language

use crate::ast::Expression::{BinaryOperation, NumberLiteral, UnaryOperation};

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Not,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    NumberLiteral(f64),
    BooleanLiteral(bool),
    Variable(String),
    BinaryOperation(Operation, Box<Expression>, Box<Expression>),
    UnaryOperation(Operation, Box<Expression>),
    Cmp(Box<Expression>, Box<Expression>),
}

/// Represents a statement in the l9 language.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Print(Expression),
    Block(Vec<Statement>),
    Declaration(String, Option<Expression>),
    Assignment(String, Expression),
    If(Expression, Box<Statement>, Option<Box<Statement>>),
    While(Expression, Box<Statement>),
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        Program { statements }
    }

    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }
}

impl Expression {
    pub fn binary(op: Operation, lhs: Expression, rhs: Expression) -> Self {
        BinaryOperation(op, Box::new(lhs), Box::new(rhs))
    }

    pub fn unary(op: Operation, lhs: Expression) -> Self {
        UnaryOperation(op, Box::new(lhs))
    }

    pub fn number(n: impl Into<f64>) -> Self {
        NumberLiteral(n.into())
    }
}

impl Statement {
    pub fn expression(expr: Expression) -> Self {
        Statement::Expression(expr)
    }

    pub fn print(expr: Expression) -> Self {
        Statement::Print(expr)
    }
}
