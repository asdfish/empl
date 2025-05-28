//! custom lisp dialect used for configurating this program

pub mod ast;
pub mod evaluator;
pub mod lexer;
pub mod parser;

use crate::config::{IntermediateConfig, Resources, clisp::evaluator::EvalError};

pub fn execute(resources: &mut Resources) -> Option<Result<IntermediateConfig, CLispError>> {
    None
}

#[derive(Debug)]
pub enum CLispError {
    Eval(EvalError),
}
