//! Command-line tokeniser.
//!
//! Splits a raw input line into arguments while respecting single and double
//! quotes. Only the basics required by the spec are implemented: no globbing,
//! no piping and no redirection.

/// Splits `input` into tokens.
///
/// * Whitespace (spaces and tabs) separates tokens.
/// * Text inside single quotes is taken literally.
/// * Text inside double quotes is taken literally (no variable expansion).
///
/// Returns an error message if a quote is left unclosed.
pub fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            '\'' | '"' => {
                in_token = true;
                let quote = c;
                let mut closed = false;
                for qc in chars.by_ref() {
                    if qc == quote {
                        closed = true;
                        break;
                    }
                    current.push(qc);
                }
                if !closed {
                    return Err(format!("unexpected EOF while looking for matching `{quote}`"));
                }
            }
            _ => {
                in_token = true;
                current.push(c);
            }
        }
    }

    if in_token {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(tokenize("ls -l -a").unwrap(), vec!["ls", "-l", "-a"]);
    }

    #[test]
    fn collapses_repeated_whitespace() {
        assert_eq!(tokenize("  echo   hi  ").unwrap(), vec!["echo", "hi"]);
    }

    #[test]
    fn keeps_double_quoted_spaces() {
        assert_eq!(
            tokenize("echo \"Hello There\"").unwrap(),
            vec!["echo", "Hello There"]
        );
    }

    #[test]
    fn keeps_single_quoted_spaces() {
        assert_eq!(tokenize("echo 'a b c'").unwrap(), vec!["echo", "a b c"]);
    }

    #[test]
    fn joins_adjacent_quoted_segments() {
        assert_eq!(tokenize("echo a'b'c").unwrap(), vec!["echo", "abc"]);
    }

    #[test]
    fn empty_input_yields_no_tokens() {
        assert!(tokenize("   ").unwrap().is_empty());
    }

    #[test]
    fn unclosed_quote_is_an_error() {
        assert!(tokenize("echo \"oops").is_err());
    }
}
