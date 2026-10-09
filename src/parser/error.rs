use std::fmt;

use pest::error::{ErrorVariant, InputLocation, LineColLocation};
use thiserror::Error;

use super::{AstNode, Location, Rule};

#[derive(Debug)]
pub struct PestErrorInfo {
    pub message: String,
    pub variant: PestErrorVariantInfo,
    pub location: InputLocation,
    pub line_col: LineColLocation,
}

#[derive(Debug)]
pub enum PestErrorVariantInfo {
    ParsingError {
        positives: Vec<Rule>,
        negatives: Vec<Rule>,
    },
    CustomError {
        message: String,
    },
}

impl fmt::Display for PestErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<pest::error::Error<Rule>> for PestErrorInfo {
    fn from(error: pest::error::Error<Rule>) -> Self {
        let message = error.to_string();
        let line_col = error.line_col;
        let location = error.location;
        let variant = match error.variant {
            ErrorVariant::ParsingError {
                positives,
                negatives,
            } => PestErrorVariantInfo::ParsingError {
                positives,
                negatives,
            },
            ErrorVariant::CustomError { message } => PestErrorVariantInfo::CustomError { message },
        };
        Self {
            message,
            variant,
            location,
            line_col,
        }
    }
}

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Invalid indentation:\n{0}")]
    InvalidIndentation(Location),
    #[error("Failed to parse:\n{1}")]
    ParseError(Location, PestErrorInfo),
    // #[error("Invalid command parameter: {0}")]
    // InvalidCommandParameter(String),
    // #[error("Unexpected token: {0}")]
    // UnexpectedToken(String),
}

impl ParserError {
    pub fn location(&self) -> &Location {
        match self {
            ParserError::InvalidIndentation(loc) => loc,
            ParserError::ParseError(loc, _) => loc,
        }
    }
}

#[derive(Debug)]
pub struct ParserResult {
    pub ast: AstNode,
    pub parse_errors: Vec<ParserError>,
}
