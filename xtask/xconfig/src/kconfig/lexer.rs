use crate::error::{KconfigError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Config,
    MenuConfig,
    Choice,
    EndChoice,
    Menu,
    EndMenu,
    If,
    EndIf,
    Source,
    Comment,
    Bool,
    Tristate,
    String,
    Int,
    Hex,
    Prompt,
    Default,
    Depends,
    Select,
    Imply,
    Range,
    Help,
    Visible,
    Option,
    On,
    MainMenu,
    Modules,
    Defconfig,
    AllNoConfig,
    AllYesConfig,
    AllModConfig,
    RandConfig,
    ListNewConfig,

    // Operators
    Eq,          // =
    NotEq,       // !=
    Less,        // <
    LessEq,      // <=
    Greater,     // >
    GreaterEq,   // >=
    And,         // &&
    Or,          // ||
    Not,         // !
    
    // Literals
    Identifier(String),
    StringLit(String),
    Number(i64),
    
    // Punctuation
    LParen,      // (
    RParen,      // )
    
    // Special
    Newline,
    Eof,
}

pub struct Lexer {
    input: String,
    position: usize,
    line: usize,
    file: PathBuf,
}

impl Lexer {
    pub fn new(input: String, file: PathBuf) -> Self {
        Self {
            input,
            position: 0,
            line: 1,
            file,
        }
    }

    pub fn current_line(&self) -> usize {
        self.line
    }

    pub fn current_file(&self) -> &PathBuf {
        &self.file
    }

    pub fn skip_help_text(&mut self) -> String {
        let mut help = String::new();
        
        // Skip any whitespace/newlines immediately after "help" keyword
        while let Some(ch) = self.current_char() {
            if ch == '\n' {
                self.advance();
                break;
            } else if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                // No newline after help, unexpected
                break;
            }
        }
        
        // Now collect all indented lines
        loop {
            // Peek at the start of the line
            let _line_start = self.position;
            
            // Check if this line is indented (starts with space or tab)
            match self.current_char() {
                None => break, // EOF
                Some('\n') => {
                    // Empty line, consume and continue
                    self.advance();
                    help.push('\n');
                    continue;
                }
                Some(' ') | Some('\t') => {
                    // Indented line, this is help text
                    // Consume the whole line
                    while let Some(ch) = self.current_char() {
                        help.push(ch);
                        self.advance();
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                _ => {
                    // Non-indented line, help text is done
                    break;
                }
            }
        }
        
        help
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.input[self.position..].chars().nth(offset)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current_char()?;
        self.position += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.current_char() == Some('#') {
            while let Some(ch) = self.current_char() {
                if ch == '\n' {
                    break;
                }
                self.advance();
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn read_string(&mut self) -> Result<String> {
        self.advance(); // skip opening quote
        let mut result = String::new();
        let mut escaped = false;

        while let Some(ch) = self.current_char() {
            if escaped {
                result.push(match ch {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    _ => ch,
                });
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == '"' {
                self.advance(); // skip closing quote
                return Ok(result);
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(KconfigError::Syntax {
            file: self.file.clone(),
            line: self.line,
            message: "Unterminated string literal".to_string(),
        })
    }

    fn read_number(&mut self) -> i64 {
        let mut result = String::new();
        
        // Handle hex numbers
        if self.current_char() == Some('0') && self.peek_char(1) == Some('x') {
            self.advance(); // 0
            self.advance(); // x
            while let Some(ch) = self.current_char() {
                if ch.is_ascii_hexdigit() {
                    result.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            return i64::from_str_radix(&result, 16).unwrap_or(0);
        }

        // Handle decimal numbers
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_digit() {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result.parse().unwrap_or(0)
    }

    pub fn next_token(&mut self) -> Result<Token> {
        loop {
            self.skip_whitespace();

            if self.current_char() == Some('#') {
                self.skip_comment();
                continue;
            }

            break;
        }

        let ch = match self.current_char() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        if ch == '\n' {
            self.advance();
            return Ok(Token::Newline);
        }

        if ch == '"' {
            return Ok(Token::StringLit(self.read_string()?));
        }

        if ch.is_ascii_digit() {
            return Ok(Token::Number(self.read_number()));
        }

        if ch == '(' {
            self.advance();
            return Ok(Token::LParen);
        }

        if ch == ')' {
            self.advance();
            return Ok(Token::RParen);
        }

        if ch == '=' {
            self.advance();
            if self.current_char() == Some('=') {
                self.advance();
                return Ok(Token::Eq);
            }
            return Ok(Token::Eq);
        }

        if ch == '!' {
            self.advance();
            if self.current_char() == Some('=') {
                self.advance();
                return Ok(Token::NotEq);
            }
            return Ok(Token::Not);
        }

        if ch == '<' {
            self.advance();
            if self.current_char() == Some('=') {
                self.advance();
                return Ok(Token::LessEq);
            }
            return Ok(Token::Less);
        }

        if ch == '>' {
            self.advance();
            if self.current_char() == Some('=') {
                self.advance();
                return Ok(Token::GreaterEq);
            }
            return Ok(Token::Greater);
        }

        if ch == '&' {
            self.advance();
            if self.current_char() == Some('&') {
                self.advance();
                return Ok(Token::And);
            }
            return Err(KconfigError::Syntax {
                file: self.file.clone(),
                line: self.line,
                message: "Expected '&&'".to_string(),
            });
        }

        if ch == '|' {
            self.advance();
            if self.current_char() == Some('|') {
                self.advance();
                return Ok(Token::Or);
            }
            return Err(KconfigError::Syntax {
                file: self.file.clone(),
                line: self.line,
                message: "Expected '||'".to_string(),
            });
        }

        if ch.is_alphabetic() || ch == '_' {
            let ident = self.read_identifier();
            let token = match ident.as_str() {
                "config" => Token::Config,
                "menuconfig" => Token::MenuConfig,
                "choice" => Token::Choice,
                "endchoice" => Token::EndChoice,
                "menu" => Token::Menu,
                "endmenu" => Token::EndMenu,
                "if" => Token::If,
                "endif" => Token::EndIf,
                "source" => Token::Source,
                "comment" => Token::Comment,
                "bool" => Token::Bool,
                "tristate" => Token::Tristate,
                "string" => Token::String,
                "int" => Token::Int,
                "hex" => Token::Hex,
                "prompt" => Token::Prompt,
                "default" => Token::Default,
                "depends" => Token::Depends,
                "select" => Token::Select,
                "imply" => Token::Imply,
                "range" => Token::Range,
                "help" => Token::Help,
                "visible" => Token::Visible,
                "option" => Token::Option,
                "on" => Token::On,
                "mainmenu" => Token::MainMenu,
                "modules" => Token::Modules,
                "defconfig_list" => Token::Defconfig,
                "allnoconfig_y" => Token::AllNoConfig,
                _ => Token::Identifier(ident),
            };
            return Ok(token);
        }

        Err(KconfigError::Syntax {
            file: self.file.clone(),
            line: self.line,
            message: format!("Unexpected character: '{}'", ch),
        })
    }

    pub fn peek_token(&mut self) -> Result<Token> {
        let saved_position = self.position;
        let saved_line = self.line;
        let token = self.next_token()?;
        self.position = saved_position;
        self.line = saved_line;
        Ok(token)
    }
}
