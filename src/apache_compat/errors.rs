//! Apache Configuration Parser Errors

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type for Apache parsing operations
pub type ParseResult<T> = Result<T, ApacheParseError>;

/// Errors that can occur while parsing Apache configuration
#[derive(Debug)]
pub enum ApacheParseError {
    /// I/O error reading file
    IoError {
        path: PathBuf,
        source: io::Error,
    },

    /// Syntax error at specific line
    SyntaxError {
        line: usize,
        message: String,
    },

    /// Empty directive
    EmptyDirective,

    /// Empty block (e.g., "<>" without content)
    EmptyBlock,

    /// Unclosed block directive
    UnclosedBlock,

    /// Unknown block type
    UnknownBlock(String),

    /// Unknown directive
    UnknownDirective(String),

    /// Invalid value for directive
    InvalidValue {
        directive: String,
        value: String,
        expected: String,
    },

    /// Missing required directive in VirtualHost
    MissingRequired {
        vhost: String,
        directive: String,
    },

    /// Invalid path
    InvalidPath {
        directive: String,
        path: String,
    },

    /// Circular include detected
    CircularInclude {
        path: PathBuf,
    },

    /// Nested block too deep
    NestingTooDeep {
        max_depth: usize,
    },
}

impl fmt::Display for ApacheParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApacheParseError::IoError { path, source } => {
                write!(f, "I/O error reading '{}': {}", path.display(), source)
            }
            ApacheParseError::SyntaxError { line, message } => {
                write!(f, "Syntax error at line {}: {}", line, message)
            }
            ApacheParseError::EmptyDirective => {
                write!(f, "Empty directive")
            }
            ApacheParseError::EmptyBlock => {
                write!(f, "Empty block directive")
            }
            ApacheParseError::UnclosedBlock => {
                write!(f, "Unclosed block directive (missing '</...>')")
            }
            ApacheParseError::UnknownBlock(block) => {
                write!(f, "Unknown block type: <{}>", block)
            }
            ApacheParseError::UnknownDirective(directive) => {
                write!(f, "Unknown directive: {}", directive)
            }
            ApacheParseError::InvalidValue {
                directive,
                value,
                expected,
            } => {
                write!(
                    f,
                    "Invalid value '{}' for directive '{}', expected: {}",
                    value, directive, expected
                )
            }
            ApacheParseError::MissingRequired { vhost, directive } => {
                write!(
                    f,
                    "VirtualHost '{}' missing required directive: {}",
                    vhost, directive
                )
            }
            ApacheParseError::InvalidPath { directive, path } => {
                write!(
                    f,
                    "Invalid path '{}' for directive '{}'",
                    path, directive
                )
            }
            ApacheParseError::CircularInclude { path } => {
                write!(f, "Circular include detected: {}", path.display())
            }
            ApacheParseError::NestingTooDeep { max_depth } => {
                write!(f, "Block nesting exceeds maximum depth of {}", max_depth)
            }
        }
    }
}

impl std::error::Error for ApacheParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApacheParseError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for ApacheParseError {
    fn from(err: io::Error) -> Self {
        ApacheParseError::IoError {
            path: PathBuf::from("<unknown>"),
            source: err,
        }
    }
}
