//! Command-line parser.
//!
//! Turns a raw input line into a list of pipelines. The grammar supported is
//! deliberately small:
//!
//! ```text
//! list     := pipeline (';' pipeline)*
//! pipeline := command ('|' command)*
//! command  := word+ (redirection)*
//! ```
//!
//! Quoting rules follow the shell conventions: single quotes are literal,
//! double quotes allow `$VAR` expansion, and a backslash escapes the next
//! character. Globbing is intentionally not implemented.

use std::env;

/// A single command with its arguments and optional redirections.
#[derive(Debug, Default, PartialEq)]
pub struct Command {
    pub argv: Vec<String>,
    /// Input redirection target (`< file`).
    pub stdin: Option<String>,
    /// Output redirection: `(path, append)` where `append` is `>>`.
    pub stdout: Option<(String, bool)>,
}

/// A sequence of commands connected by pipes.
#[derive(Debug, Default, PartialEq)]
pub struct Pipeline {
    pub commands: Vec<Command>,
}

/// The internal token stream produced by the lexer.
#[derive(Debug, PartialEq)]
enum Token {
    Word(String),
    Semicolon,
    Pipe,
    Great,  // >
    DGreat, // >>
    Less,   // <
}

/// Parses a full input line into a list of pipelines.
///
/// Empty statements (e.g. a trailing `;`) are skipped. Returns a syntax-error
/// message on malformed input such as an unterminated quote or a redirection
/// without a target.
pub fn parse(input: &str) -> Result<Vec<Pipeline>, String> {
    let tokens = tokenize(input)?;
    build_pipelines(tokens)
}

/// Splits the tokens on `;` and builds one pipeline per statement.
fn build_pipelines(tokens: Vec<Token>) -> Result<Vec<Pipeline>, String> {
    let mut pipelines = Vec::new();
    let mut statement: Vec<Token> = Vec::new();

    for token in tokens {
        match token {
            Token::Semicolon => {
                if !statement.is_empty() {
                    pipelines.push(build_pipeline(std::mem::take(&mut statement))?);
                }
            }
            other => statement.push(other),
        }
    }
    if !statement.is_empty() {
        pipelines.push(build_pipeline(statement)?);
    }

    Ok(pipelines)
}

/// Builds a single pipeline, splitting on `|` and attaching redirections.
fn build_pipeline(tokens: Vec<Token>) -> Result<Pipeline, String> {
    let mut pipeline = Pipeline::default();
    let mut current = Command::default();
    let mut tokens = tokens.into_iter().peekable();

    // A closure would need to borrow `current` mutably alongside the iterator,
    // so we keep an explicit loop instead.
    while let Some(token) = tokens.next() {
        match token {
            Token::Word(w) => current.argv.push(w),
            Token::Pipe => {
                finish_command(&mut pipeline, std::mem::take(&mut current))?;
            }
            Token::Less => {
                current.stdin = Some(expect_filename(tokens.next(), "<")?);
            }
            Token::Great => {
                current.stdout = Some((expect_filename(tokens.next(), ">")?, false));
            }
            Token::DGreat => {
                current.stdout = Some((expect_filename(tokens.next(), ">>")?, true));
            }
            Token::Semicolon => unreachable!("semicolons are split out earlier"),
        }
    }

    finish_command(&mut pipeline, current)?;
    Ok(pipeline)
}

/// Pushes a completed command onto the pipeline, rejecting empty commands
/// (which would come from `| |` or a leading/trailing pipe).
fn finish_command(pipeline: &mut Pipeline, command: Command) -> Result<(), String> {
    if command.argv.is_empty() {
        return Err("syntax error near unexpected token `|`".to_string());
    }
    pipeline.commands.push(command);
    Ok(())
}

/// Extracts the filename that must follow a redirection operator.
fn expect_filename(token: Option<Token>, op: &str) -> Result<String, String> {
    match token {
        Some(Token::Word(name)) => Ok(name),
        _ => Err(format!("syntax error near unexpected token after `{op}`")),
    }
}

/// The lexer: converts the input string into a token stream, performing quote
/// handling and `$VAR` expansion along the way.
fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut has_word = false; // distinguishes an empty quoted word ("") from no word
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => {
                flush_word(&mut tokens, &mut word, &mut has_word);
                i += 1;
            }
            ';' => {
                flush_word(&mut tokens, &mut word, &mut has_word);
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '|' => {
                flush_word(&mut tokens, &mut word, &mut has_word);
                tokens.push(Token::Pipe);
                i += 1;
            }
            '<' => {
                flush_word(&mut tokens, &mut word, &mut has_word);
                tokens.push(Token::Less);
                i += 1;
            }
            '>' => {
                flush_word(&mut tokens, &mut word, &mut has_word);
                if chars.get(i + 1) == Some(&'>') {
                    tokens.push(Token::DGreat);
                    i += 2;
                } else {
                    tokens.push(Token::Great);
                    i += 1;
                }
            }
            '\'' => {
                has_word = true;
                i += 1;
                let closed = read_single_quoted(&chars, &mut i, &mut word);
                if !closed {
                    return Err("unexpected EOF while looking for matching `'`".to_string());
                }
            }
            '"' => {
                has_word = true;
                i += 1;
                let closed = read_double_quoted(&chars, &mut i, &mut word);
                if !closed {
                    return Err("unexpected EOF while looking for matching `\"`".to_string());
                }
            }
            '$' => {
                has_word = true;
                word.push_str(&read_variable(&chars, &mut i));
            }
            '\\' => {
                has_word = true;
                // A backslash quotes the following character literally.
                if let Some(&next) = chars.get(i + 1) {
                    word.push(next);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                has_word = true;
                word.push(c);
                i += 1;
            }
        }
    }

    flush_word(&mut tokens, &mut word, &mut has_word);
    Ok(tokens)
}

/// Emits the accumulated word as a token, if any characters were collected.
fn flush_word(tokens: &mut Vec<Token>, word: &mut String, has_word: &mut bool) {
    if *has_word {
        tokens.push(Token::Word(std::mem::take(word)));
        *has_word = false;
    }
}

/// Reads a single-quoted segment verbatim. Returns whether the closing quote
/// was found. `i` points just past the opening quote on entry.
fn read_single_quoted(chars: &[char], i: &mut usize, word: &mut String) -> bool {
    while *i < chars.len() {
        let c = chars[*i];
        *i += 1;
        if c == '\'' {
            return true;
        }
        word.push(c);
    }
    false
}

/// Reads a double-quoted segment, expanding `$VAR` and honouring backslash
/// escapes for `"`, `\` and `$`. `i` points just past the opening quote.
fn read_double_quoted(chars: &[char], i: &mut usize, word: &mut String) -> bool {
    while *i < chars.len() {
        match chars[*i] {
            '"' => {
                *i += 1;
                return true;
            }
            '$' => word.push_str(&read_variable(chars, i)),
            '\\' => {
                if let Some(&next) = chars.get(*i + 1) {
                    if matches!(next, '"' | '\\' | '$') {
                        word.push(next);
                        *i += 2;
                        continue;
                    }
                }
                word.push('\\');
                *i += 1;
            }
            c => {
                word.push(c);
                *i += 1;
            }
        }
    }
    false
}

/// Reads a variable reference beginning at `chars[i] == '$'` and returns its
/// value from the environment. Supports `$NAME` and `${NAME}`; a lone `$`, or
/// `$` followed by an invalid name, is treated as a literal `$`.
fn read_variable(chars: &[char], i: &mut usize) -> String {
    *i += 1; // consume '$'

    // Braced form: ${NAME}
    if chars.get(*i) == Some(&'{') {
        *i += 1;
        let mut name = String::new();
        while let Some(&c) = chars.get(*i) {
            if c == '}' {
                *i += 1;
                return lookup(&name);
            }
            name.push(c);
            *i += 1;
        }
        // Unterminated brace: treat what we saw literally.
        return format!("${{{name}");
    }

    // Bare form: $NAME where NAME is [A-Za-z_][A-Za-z0-9_]*
    let mut name = String::new();
    while let Some(&c) = chars.get(*i) {
        let valid = c == '_' || c.is_ascii_alphanumeric();
        let first = name.is_empty();
        if valid && !(first && c.is_ascii_digit()) {
            name.push(c);
            *i += 1;
        } else {
            break;
        }
    }

    if name.is_empty() {
        "$".to_string()
    } else {
        lookup(&name)
    }
}

/// Looks up an environment variable, yielding the empty string when unset.
fn lookup(name: &str) -> String {
    env::var(name).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(input: &str) -> Vec<String> {
        let pipelines = parse(input).unwrap();
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].commands.len(), 1);
        pipelines[0].commands[0].argv.clone()
    }

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(words("ls -l -a"), vec!["ls", "-l", "-a"]);
    }

    #[test]
    fn keeps_double_quoted_spaces() {
        assert_eq!(words("echo \"Hello There\""), vec!["echo", "Hello There"]);
    }

    #[test]
    fn keeps_single_quoted_spaces() {
        assert_eq!(words("echo 'a b c'"), vec!["echo", "a b c"]);
    }

    #[test]
    fn joins_adjacent_quoted_segments() {
        assert_eq!(words("echo a'b'c"), vec!["echo", "abc"]);
    }

    #[test]
    fn single_quotes_do_not_expand() {
        std::env::set_var("ZS_TEST_VAR", "value");
        assert_eq!(words("echo '$ZS_TEST_VAR'"), vec!["echo", "$ZS_TEST_VAR"]);
    }

    #[test]
    fn double_quotes_expand_variables() {
        std::env::set_var("ZS_TEST_VAR", "value");
        assert_eq!(words("echo \"$ZS_TEST_VAR\""), vec!["echo", "value"]);
    }

    #[test]
    fn braced_variable_expansion() {
        std::env::set_var("ZS_TEST_VAR", "value");
        assert_eq!(words("echo ${ZS_TEST_VAR}s"), vec!["echo", "values"]);
    }

    #[test]
    fn unset_variable_is_empty() {
        assert_eq!(words("echo x${ZS_DEFINITELY_UNSET}y"), vec!["echo", "xy"]);
    }

    #[test]
    fn backslash_escapes_next_char() {
        assert_eq!(words("echo a\\ b"), vec!["echo", "a b"]);
    }

    #[test]
    fn semicolon_splits_statements() {
        let pipelines = parse("echo a ; echo b").unwrap();
        assert_eq!(pipelines.len(), 2);
        assert_eq!(pipelines[0].commands[0].argv, vec!["echo", "a"]);
        assert_eq!(pipelines[1].commands[0].argv, vec!["echo", "b"]);
    }

    #[test]
    fn trailing_semicolon_is_ignored() {
        let pipelines = parse("echo a ;").unwrap();
        assert_eq!(pipelines.len(), 1);
    }

    #[test]
    fn pipe_builds_multiple_commands() {
        let pipelines = parse("cat file | cat").unwrap();
        assert_eq!(pipelines[0].commands.len(), 2);
    }

    #[test]
    fn parses_output_redirection() {
        let pipelines = parse("echo hi > out.txt").unwrap();
        let cmd = &pipelines[0].commands[0];
        assert_eq!(cmd.argv, vec!["echo", "hi"]);
        assert_eq!(cmd.stdout, Some(("out.txt".to_string(), false)));
    }

    #[test]
    fn parses_append_redirection() {
        let pipelines = parse("echo hi >> out.txt").unwrap();
        assert_eq!(
            pipelines[0].commands[0].stdout,
            Some(("out.txt".to_string(), true))
        );
    }

    #[test]
    fn parses_input_redirection() {
        let pipelines = parse("cat < in.txt").unwrap();
        assert_eq!(pipelines[0].commands[0].stdin, Some("in.txt".to_string()));
    }

    #[test]
    fn unclosed_quote_is_an_error() {
        assert!(parse("echo \"oops").is_err());
    }

    #[test]
    fn redirection_without_target_is_an_error() {
        assert!(parse("echo hi >").is_err());
    }

    #[test]
    fn leading_pipe_is_an_error() {
        assert!(parse("| cat").is_err());
    }

    #[test]
    fn empty_input_yields_no_pipelines() {
        assert!(parse("   ").unwrap().is_empty());
    }
}
