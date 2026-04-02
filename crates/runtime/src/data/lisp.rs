//! Lisp script parser for Abuse startup and object definition files.
//!
//! Abuse uses a subset of Lisp for:
//! - Startup configuration (`addon/twist/startup.lsp`)
//! - Object behavior definitions (`.lsp` files in `data/lisp/`)
//! - Level scripting and event handlers
//!
//! This parser handles basic forms (lists, symbols, strings, quotes) and
//! comments. It does not evaluate or execute code; it only parses the
//! structure for later interpretation.

use std::path::{Path, PathBuf};

use thiserror::Error;
use winnow::combinator::{alt, delimited, eof, preceded, repeat, terminated};
use winnow::error::ContextError;
use winnow::token::{any, take_till};
use winnow::{ModalResult, Parser};

/// Errors that can occur during Lisp parsing or loading.
#[derive(Debug, Error)]
pub enum LispError {
    #[error("failed to read script at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unexpected token in input")]
    UnexpectedToken,
    #[error("unterminated string in input")]
    UnterminatedString,
    #[error("unterminated list in input")]
    UnterminatedList,
}

/// A parsed Lisp program consisting of top-level forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LispProgram {
    pub forms: Vec<LispExpr>,
}

/// A single Lisp expression (form).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LispExpr {
    /// A list of sub-expressions: `(defun foo (x) (+ x 1))`
    List(Vec<LispExpr>),
    /// An unquoted symbol: `foo`, `+`, `defun`
    Symbol(String),
    /// A quoted string literal: `"hello"`
    String(String),
    /// A quoted expression: `'foo` or `'(1 2 3)`
    Quote(Box<LispExpr>),
}

impl LispProgram {
    /// Parse a Lisp program from source text.
    ///
    /// Returns a `LispProgram` containing all top-level forms, or an error
    /// if the input is malformed.
    pub fn parse(source: &str) -> Result<Self, LispError> {
        let mut input = source;
        let mut parser = terminated(program, eof);

        match parser.parse_next(&mut input) {
            Ok(forms) => Ok(Self { forms }),
            Err(_) => {
                if has_unterminated_string(source) {
                    return Err(LispError::UnterminatedString);
                }
                if has_unmatched_list(source) {
                    return Err(LispError::UnterminatedList);
                }
                Err(LispError::UnexpectedToken)
            }
        }
    }

    /// Load and parse a Lisp script file from disk.
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, LispError> {
        let path_ref = path.as_ref();
        let source = std::fs::read_to_string(path_ref).map_err(|source| LispError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;

        Self::parse(&source)
    }

    /// Extract all `(load "path")` targets from the program.
    ///
    /// This is used to discover transitive script dependencies during startup.
    pub fn collect_load_targets(&self) -> Vec<String> {
        let mut out = Vec::new();
        for form in &self.forms {
            if let LispExpr::List(items) = form
                && let [LispExpr::Symbol(head), LispExpr::String(target)] = items.as_slice()
                && head == "load"
            {
                out.push(target.clone());
            }
        }
        out
    }
}

fn program(input: &mut &str) -> ModalResult<Vec<LispExpr>, ContextError> {
    preceded(
        ws_and_comments,
        repeat(0.., terminated(expr, ws_and_comments)),
    )
    .parse_next(input)
}

fn expr(input: &mut &str) -> ModalResult<LispExpr, ContextError> {
    preceded(
        ws_and_comments,
        alt((list_expr, quote_expr, string_expr, symbol_expr)),
    )
    .parse_next(input)
}

fn list_expr(input: &mut &str) -> ModalResult<LispExpr, ContextError> {
    delimited(
        '(',
        repeat(0.., terminated(expr, ws_and_comments)),
        preceded(ws_and_comments, ')'),
    )
    .map(LispExpr::List)
    .parse_next(input)
}

fn quote_expr(input: &mut &str) -> ModalResult<LispExpr, ContextError> {
    preceded('\'', expr)
        .map(|inner| LispExpr::Quote(Box::new(inner)))
        .parse_next(input)
}

fn string_expr(input: &mut &str) -> ModalResult<LispExpr, ContextError> {
    delimited('"', string_body, '"')
        .map(LispExpr::String)
        .parse_next(input)
}

fn string_body(input: &mut &str) -> ModalResult<String, ContextError> {
    let mut output = String::new();

    loop {
        if input.is_empty() || input.starts_with('"') {
            return Ok(output);
        }

        let ch = any.parse_next(input)?;
        if ch == '\\' {
            let esc = any.parse_next(input)?;
            match esc {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                other => output.push(other),
            }
        } else {
            output.push(ch);
        }
    }
}

fn symbol_expr(input: &mut &str) -> ModalResult<LispExpr, ContextError> {
    take_till(1.., is_symbol_terminator)
        .map(|s: &str| LispExpr::Symbol(s.to_string()))
        .parse_next(input)
}

fn ws_and_comments(input: &mut &str) -> ModalResult<(), ContextError> {
    loop {
        let before = *input;

        take_till(0.., |c: char| !c.is_ascii_whitespace())
            .void()
            .parse_next(input)?;

        if input.starts_with(';') {
            preceded(';', take_till(0.., |c| c == '\n')).parse_next(input)?;
            if input.starts_with('\n') {
                any.void().parse_next(input)?;
            }
            continue;
        }

        if *input == before {
            break;
        }
    }

    Ok(())
}

fn is_symbol_terminator(c: char) -> bool {
    c.is_ascii_whitespace() || matches!(c, '(' | ')' | ';' | '"' | '\'')
}

fn has_unterminated_string(source: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for ch in source.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == ';' {
            in_comment = true;
        } else if ch == '"' {
            in_string = true;
        }
    }

    in_string
}

fn has_unmatched_list(source: &str) -> bool {
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for ch in source.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            ';' => in_comment = true,
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }

        if depth < 0 {
            return true;
        }
    }

    depth != 0
}

#[cfg(test)]
mod tests {
    use super::{LispExpr, LispProgram};

    #[test]
    fn parses_basic_forms() {
        let source = "(perm-space)\n(setq section 'game_section)\n(load \"lisp/common.lsp\")\n";
        let program = LispProgram::parse(source).expect("program should parse");

        assert_eq!(program.forms.len(), 3);
        assert_eq!(program.collect_load_targets(), vec!["lisp/common.lsp"]);

        let second = &program.forms[1];
        match second {
            LispExpr::List(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn ignores_line_comments() {
        let source = "; comment\n(load \"a.lsp\") ; trailing\n(load \"b.lsp\")\n";
        let program = LispProgram::parse(source).expect("program should parse");
        assert_eq!(program.collect_load_targets(), vec!["a.lsp", "b.lsp"]);
    }

    #[test]
    fn fails_on_unterminated_string() {
        let source = "(load \"broken)";
        let err = LispProgram::parse(source).expect_err("expected parse error");
        let message = err.to_string();
        assert!(message.contains("unterminated string"));
    }
}
