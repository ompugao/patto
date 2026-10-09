use pest_derive::Parser;

mod ast;
mod command;
mod deadline;
mod document;
mod error;
mod inline;
mod property;

pub use ast::{
    Annotation, AstNode, AstNodeInternal, AstNodeKind, Location, Property, Span, TaskStatus,
};
pub use deadline::{parse_deadline_pub, Deadline};
pub use document::{parse_text, parse_text_with_persistent_line_tracking};
pub use error::{ParserError, ParserResult, PestErrorInfo, PestErrorVariantInfo};

#[derive(Parser)]
#[grammar = "patto.pest"]
pub struct PattoLineParser;
