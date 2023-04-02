//! Parser for the l9 interpreter

use std::iter::Peekable;

use log::trace;
use thiserror::Error;

use crate::ast::{Expression, Operation, Program, Statement};
use crate::lexer::{SourceToken, Token};
use crate::source::Position;

#[derive(Debug)]
pub struct Parser<T>
where
    T: Iterator<Item = SourceToken>,
{
    tokens: Peekable<T>,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ParsingError {
    #[error("error during parsing at {0}")]
    Unknown(Position),
    #[error("unexpected token `{0:?}` at {1}")]
    UnexpectedToken(Token, Position),
    #[error("missing operand at {0}")]
    MissingOperand(Position),
    #[error("unknown operation at {0}")]
    UnknownOperation(Position),
    #[error("missing closing parentheses at {0}")]
    MissingClosingParentheses(Position),
}

impl<T> Parser<T>
where
    T: Iterator<Item = SourceToken>,
{
    pub fn new(tokens: T) -> Self {
        Parser {
            tokens: tokens.peekable(),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParsingError> {
        let mut program = Program::default();
        while self.tokens.peek().is_some() {
            program.add_statement(self.statement()?);
        }
        Ok(program)
    }

    pub fn parse(&mut self) -> Result<Statement, ParsingError> {
        self.statement()
    }

    fn statement(&mut self) -> Result<Statement, ParsingError> {
        let token = self.advance();
        match token.kind() {
            Token::Print => {
                trace!("Parsing print statement");
                let expr = self.expression(0)?;
                self.consume(Token::Semicolon)?;
                Ok(Statement::Print(expr))
            }
            Token::LeftCurly => self.block_statement(),
            Token::Let => {
                let declaration = self.variable_declaration();
                self.consume(Token::Semicolon)?;
                declaration
            }
            Token::If => self.if_statement(),
            Token::While => self.while_statement(),
            Token::Identifier(name) => {
                let assignment = self.variable_assignment(&token, name);
                self.consume(Token::Semicolon)?;
                assignment
            }
            _ => Err(ParsingError::Unknown(*token.source())),
        }
    }

    fn variable_assignment(
        &mut self,
        token: &SourceToken,
        name: &str,
    ) -> Result<Statement, ParsingError> {
        match self.advance().kind() {
            Token::Equal => {
                let expr = self.expression(0)?;
                Ok(Statement::Assignment(name.to_string(), expr))
            }
            _ => Err(ParsingError::Unknown(*token.source())),
        }
    }

    fn expression(&mut self, min_binding: u8) -> Result<Expression, ParsingError> {
        trace!("Parsing expression (min_binding: {})", min_binding);
        let token = self.advance();
        trace!("Parsing expression (token: {:?})", token);
        let mut lhs = match token.kind() {
            Token::Number(n) => Expression::number(*n),
            Token::True => Expression::BooleanLiteral(true),
            Token::False => Expression::BooleanLiteral(false),
            Token::Minus => {
                let binding = self.prefix_binding(token.clone())?;
                let rhs = self.expression(binding)?;
                Expression::unary(Operation::Sub, rhs)
            }
            Token::Bang => {
                let binding = self.prefix_binding(token.clone())?;
                let rhs = self.expression(binding)?;
                Expression::unary(Operation::Not, rhs)
            }
            Token::Identifier(name) => Expression::Variable(name.clone()),
            Token::LParen => {
                let expr = self.expression(0)?;
                match self.advance().kind() {
                    Token::RParen => expr,
                    _ => return Err(ParsingError::MissingClosingParentheses(*token.source())),
                }
            }
            t => return Err(ParsingError::UnexpectedToken(t.clone(), *token.source())),
        };

        loop {
            let token = self.peek();
            let op = match token.kind() {
                Token::Plus => Operation::Add,
                Token::Minus => Operation::Sub,
                Token::Star => Operation::Mul,
                Token::Slash => Operation::Div,
                Token::EqualEqual => Operation::Equal,
                Token::BangEqual => Operation::NotEqual,
                Token::Less => Operation::Less,
                Token::LessEqual => Operation::LessOrEqual,
                Token::Greater => Operation::Greater,
                Token::GreaterEqual => Operation::GreaterOrEqual,
                Token::EndOfFile | Token::RParen | Token::Semicolon => break,
                _ => return Err(ParsingError::UnknownOperation(*token.source())),
            };

            if let Some((left_binding, right_binding)) = self.infix_binding(&op) {
                if left_binding < min_binding {
                    break;
                }

                self.advance();
                let rhs = self
                    .expression(right_binding)
                    .map_err(|_| ParsingError::MissingOperand(*token.source()))?;
                lhs = Expression::binary(op, lhs, rhs);

                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn block_statement(&mut self) -> Result<Statement, ParsingError> {
        trace!("Parsing block statement");
        let mut statements = Vec::new();
        loop {
            match self.peek().kind() {
                Token::RightCurly | Token::EndOfFile => {
                    break;
                }
                _ => {}
            }
            statements.push(self.statement()?);
        }
        self.consume(Token::RightCurly)?;
        Ok(Statement::Block(statements))
    }

    fn infix_binding(&self, op: &Operation) -> Option<(u8, u8)> {
        match op {
            Operation::Add | Operation::Sub => Some((3, 4)),
            Operation::Mul | Operation::Div => Some((5, 6)),
            Operation::Equal | Operation::NotEqual => Some((1, 2)),
            Operation::Less | Operation::LessOrEqual => Some((1, 2)),
            Operation::Greater | Operation::GreaterOrEqual => Some((1, 2)),
            Operation::Not => None,
        }
    }

    fn prefix_binding(&self, token: SourceToken) -> Result<u8, ParsingError> {
        match token.kind() {
            Token::Minus | Token::Bang => Ok(7),
            _ => Err(ParsingError::UnknownOperation(*token.source())),
        }
    }

    fn variable_declaration(&mut self) -> Result<Statement, ParsingError> {
        trace!("Parsing variable declaration");
        let token = self.advance();
        trace!("Variable declaration token: {:?}", token);
        let name = match token.kind() {
            Token::Identifier(name) => name,
            _ => {
                return Err(ParsingError::UnexpectedToken(
                    token.kind().clone(),
                    *token.source(),
                ))
            }
        };

        trace!("Assigning variable: {:?}", token);
        if self.advance_if(Token::Equal) {
            let expr = self.expression(0)?;
            Ok(Statement::Declaration(name.clone(), Some(expr)))
        } else {
            Ok(Statement::Declaration(name.clone(), None))
        }
    }

    fn if_statement(&mut self) -> Result<Statement, ParsingError> {
        trace!("Parsing if statement");
        self.consume(Token::LParen)?;
        let condition = self.expression(0)?;
        self.consume(Token::RParen)?;
        let then_branch = self.statement()?;
        let else_branch = if self.advance_if(Token::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        Ok(Statement::If(condition, Box::new(then_branch), else_branch))
    }

    fn while_statement(&mut self) -> Result<Statement, ParsingError> {
        trace!("Parsing while statement");
        self.consume(Token::LParen)?;
        let condition = self.expression(0)?;
        self.consume(Token::RParen)?;
        let body = self.statement()?;
        Ok(Statement::While(condition, Box::new(body)))
    }

    fn advance(&mut self) -> SourceToken {
        self.tokens
            .next()
            .unwrap_or(SourceToken::from(Token::EndOfFile))
    }

    fn advance_if(&mut self, token: Token) -> bool {
        self.tokens.next_if(|t| t.kind() == &token).is_some()
    }

    fn peek(&mut self) -> SourceToken {
        self.tokens
            .peek()
            .cloned()
            .unwrap_or(SourceToken::from(Token::EndOfFile))
    }

    fn consume(&mut self, expected: Token) -> Result<(), ParsingError> {
        trace!("Consuming token: {:?}", expected);
        let token = self.advance();
        trace!("Consuming token: current {:?}", token);
        if *token.kind() == expected {
            Ok(())
        } else {
            Err(ParsingError::UnexpectedToken(
                token.kind().clone(),
                *token.source(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Operation;

    use super::*;

    #[test]
    fn number_literal() {
        let tokens = vec![Token::Number(42.0).into()].into_iter();
        let mut parser = Parser::new(tokens);

        let ast = parser.expression(0).unwrap();

        assert_eq!(ast, Expression::NumberLiteral(42.0));
    }

    #[test]
    fn addition_expression() {
        let tokens = vec![
            Token::Number(7.0).into(),
            Token::Plus.into(),
            Token::Number(8.0).into(),
        ]
        .into_iter();
        let mut parser = Parser::new(tokens);

        let ast = parser.expression(0).unwrap();

        assert_eq!(
            ast,
            Expression::binary(Operation::Add, Expression::number(7), Expression::number(8))
        );
    }

    #[test]
    fn same_priority_operation_expression() {
        let tokens = vec![
            Token::Number(5.0).into(),
            Token::Plus.into(),
            Token::Number(10.0).into(),
            Token::Minus.into(),
            Token::Number(15.0).into(),
        ]
        .into_iter();
        let mut parser = Parser::new(tokens);

        let ast = parser.expression(0).unwrap();

        assert_eq!(
            ast,
            Expression::binary(
                Operation::Sub,
                Expression::binary(
                    Operation::Add,
                    Expression::number(5),
                    Expression::number(10),
                ),
                Expression::number(15),
            )
        );
    }

    #[test]
    fn missing_rhs_infix_operation() {
        let tokens = vec![
            SourceToken::from(Token::Number(7.0)),
            SourceToken::from(Token::Plus),
        ]
        .into_iter();
        let mut parser = Parser::new(tokens);

        let parsing_error = parser.expression(0);

        assert!(matches!(
            parsing_error,
            Err(ParsingError::MissingOperand(_))
        ));
    }

    #[test]
    fn print_variable() {
        let tokens = vec![
            Token::Print.into(),
            Token::Identifier("x".to_string()).into(),
            Token::Semicolon.into(),
        ]
        .into_iter();
        let mut parser = Parser::new(tokens);

        let ast = parser.statement().unwrap();

        assert_eq!(ast, Statement::Print(Expression::Variable("x".to_string())));
    }
}
