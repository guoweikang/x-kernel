use crate::error::{KconfigError, Result};
use crate::kconfig::ast::*;
use crate::kconfig::lexer::{Lexer, Token};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Parser {
    current_file: PathBuf,
    srctree: PathBuf,
    file_stack: Vec<FileContext>,
    parsed_files: HashSet<PathBuf>,
    inclusion_chain: Vec<PathBuf>,
}

#[allow(dead_code)]
struct FileContext {
    file_path: PathBuf,
    lexer: Lexer,
    current_token: Token,
}

impl Parser {
    pub fn new(kconfig_path: impl AsRef<Path>, srctree: impl AsRef<Path>) -> Result<Self> {
        let kconfig_path = kconfig_path.as_ref().to_path_buf();
        let srctree = srctree.as_ref().to_path_buf();
        
        if !kconfig_path.exists() {
            return Err(KconfigError::FileNotFound(kconfig_path));
        }

        let content = fs::read_to_string(&kconfig_path)?;
        let mut lexer = Lexer::new(content, kconfig_path.clone());
        let current_token = lexer.next_token()?;

        let mut parsed_files = HashSet::new();
        parsed_files.insert(kconfig_path.clone());

        Ok(Self {
            current_file: kconfig_path.clone(),
            srctree,
            file_stack: vec![FileContext {
                file_path: kconfig_path.clone(),
                lexer,
                current_token,
            }],
            parsed_files,
            inclusion_chain: vec![kconfig_path],
        })
    }

    fn current_context(&self) -> &FileContext {
        self.file_stack.last().expect("File stack is empty")
    }

    fn current_context_mut(&mut self) -> &mut FileContext {
        self.file_stack.last_mut().expect("File stack is empty")
    }

    fn advance(&mut self) -> Result<()> {
        let ctx = self.current_context_mut();
        ctx.current_token = ctx.lexer.next_token()?;
        Ok(())
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        let current = self.current_context().current_token.clone();
        if std::mem::discriminant(&current) != std::mem::discriminant(&expected) {
            return Err(KconfigError::Syntax {
                file: self.current_file.clone(),
                line: self.current_context().lexer.current_line(),
                message: format!("Expected {:?}, got {:?}", expected, current),
            });
        }
        self.advance()
    }

    fn skip_newlines(&mut self) -> Result<()> {
        while matches!(self.current_context().current_token, Token::Newline) {
            self.advance()?;
        }
        Ok(())
    }

    // Handle source directive with recursion detection
    fn handle_source(&mut self, path_expr: String) -> Result<Vec<Entry>> {
        // Resolve the path relative to srctree
        let source_path = self.srctree.join(&path_expr);
        
        // Check if file exists
        if !source_path.exists() {
            return Err(KconfigError::FileNotFound(source_path));
        }

        // Check for circular dependency
        if self.inclusion_chain.contains(&source_path) {
            let chain = self
                .inclusion_chain
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(KconfigError::RecursiveSource {
                chain: format!("{} -> {}", chain, source_path.display()),
            });
        }

        // Check if file was already parsed (but not in current chain - that's circular)
        if self.parsed_files.contains(&source_path) {
            // Already parsed, skip
            return Ok(vec![]);
        }

        // Mark as parsed
        self.parsed_files.insert(source_path.clone());
        self.inclusion_chain.push(source_path.clone());

        // Read the source file
        let content = fs::read_to_string(&source_path)?;
        let mut lexer = Lexer::new(content, source_path.clone());
        let current_token = lexer.next_token()?;

        // Push new file context
        let old_file = self.current_file.clone();
        self.current_file = source_path.clone();
        self.file_stack.push(FileContext {
            file_path: source_path.clone(),
            lexer,
            current_token,
        });

        // Parse the source file
        let entries = self.parse_entries()?;

        // Pop file context
        self.file_stack.pop();
        self.current_file = old_file;
        self.inclusion_chain.pop();

        Ok(entries)
    }

    pub fn parse(&mut self) -> Result<KconfigFile> {
        let entries = self.parse_entries()?;
        Ok(KconfigFile {
            path: self.inclusion_chain[0].clone(),
            entries,
        })
    }

    fn parse_entries(&mut self) -> Result<Vec<Entry>> {
        let mut entries = Vec::new();

        self.skip_newlines()?;

        while !matches!(self.current_context().current_token, Token::Eof) {
            match &self.current_context().current_token.clone() {
                Token::Config => {
                    entries.push(Entry::Config(self.parse_config()?));
                }
                Token::MenuConfig => {
                    entries.push(Entry::MenuConfig(self.parse_menuconfig()?));
                }
                Token::Choice => {
                    entries.push(Entry::Choice(self.parse_choice()?));
                }
                Token::Menu => {
                    entries.push(Entry::Menu(self.parse_menu()?));
                }
                Token::If => {
                    entries.push(Entry::If(self.parse_if()?));
                }
                Token::Source => {
                    self.advance()?; // consume 'source'
                    let path = self.parse_string()?;
                    self.skip_newlines()?;
                    
                    // Recursively parse the source file
                    let source_entries = self.handle_source(path.clone())?;
                    entries.extend(source_entries);
                    
                    // Also add the source entry itself
                    entries.push(Entry::Source(Source {
                        path: PathBuf::from(path),
                    }));
                }
                Token::Comment => {
                    entries.push(Entry::Comment(self.parse_comment()?));
                }
                Token::MainMenu => {
                    self.advance()?; // consume 'mainmenu'
                    let title = self.parse_string()?;
                    self.skip_newlines()?;
                    entries.push(Entry::MainMenu(title));
                }
                Token::EndMenu | Token::EndIf | Token::EndChoice => {
                    // End of block
                    break;
                }
                Token::Newline => {
                    self.advance()?;
                }
                Token::Eof => break,
                _ => {
                    return Err(KconfigError::Syntax {
                        file: self.current_file.clone(),
                        line: self.current_context().lexer.current_line(),
                        message: format!(
                            "Unexpected token: {:?}",
                            self.current_context().current_token
                        ),
                    });
                }
            }
        }

        Ok(entries)
    }

    fn parse_config(&mut self) -> Result<Config> {
        self.advance()?; // consume 'config'
        
        let name = match &self.current_context().current_token {
            Token::Identifier(s) => s.clone(),
            _ => {
                return Err(KconfigError::Syntax {
                    file: self.current_file.clone(),
                    line: self.current_context().lexer.current_line(),
                    message: "Expected identifier after 'config'".to_string(),
                });
            }
        };
        self.advance()?;
        self.skip_newlines()?;

        let (symbol_type, properties) = self.parse_config_options()?;

        Ok(Config {
            name,
            symbol_type,
            properties,
        })
    }

    fn parse_menuconfig(&mut self) -> Result<MenuConfig> {
        self.advance()?; // consume 'menuconfig'
        
        let name = match &self.current_context().current_token {
            Token::Identifier(s) => s.clone(),
            _ => {
                return Err(KconfigError::Syntax {
                    file: self.current_file.clone(),
                    line: self.current_context().lexer.current_line(),
                    message: "Expected identifier after 'menuconfig'".to_string(),
                });
            }
        };
        self.advance()?;
        self.skip_newlines()?;

        let (symbol_type, properties) = self.parse_config_options()?;

        Ok(MenuConfig {
            name,
            symbol_type,
            properties,
        })
    }

    fn parse_config_options(&mut self) -> Result<(SymbolType, Property)> {
        let mut symbol_type = SymbolType::Bool;
        let mut properties = Property::default();

        while !matches!(
            self.current_context().current_token,
            Token::Config
                | Token::MenuConfig
                | Token::Choice
                | Token::Menu
                | Token::EndMenu
                | Token::If
                | Token::EndIf
                | Token::Source
                | Token::Comment
                | Token::EndChoice
                | Token::Eof
        ) {
            match &self.current_context().current_token.clone() {
                Token::Bool => {
                    self.advance()?;
                    symbol_type = SymbolType::Bool;
                    if let Ok(prompt) = self.try_parse_prompt() {
                        properties.prompt = Some(prompt);
                    }
                }
                Token::Tristate => {
                    self.advance()?;
                    symbol_type = SymbolType::Tristate;
                    if let Ok(prompt) = self.try_parse_prompt() {
                        properties.prompt = Some(prompt);
                    }
                }
                Token::String => {
                    self.advance()?;
                    symbol_type = SymbolType::String;
                    if let Ok(prompt) = self.try_parse_prompt() {
                        properties.prompt = Some(prompt);
                    }
                }
                Token::Int => {
                    self.advance()?;
                    symbol_type = SymbolType::Int;
                    if let Ok(prompt) = self.try_parse_prompt() {
                        properties.prompt = Some(prompt);
                    }
                }
                Token::Hex => {
                    self.advance()?;
                    symbol_type = SymbolType::Hex;
                    if let Ok(prompt) = self.try_parse_prompt() {
                        properties.prompt = Some(prompt);
                    }
                }
                Token::Prompt => {
                    self.advance()?;
                    properties.prompt = Some(self.parse_string()?);
                    if matches!(self.current_context().current_token, Token::If) {
                        self.advance()?;
                        // Parse if condition (simplified)
                    }
                }
                Token::Default => {
                    self.advance()?;
                    properties.default = Some(self.parse_expr()?);
                }
                Token::Depends => {
                    self.advance()?;
                    self.expect(Token::On)?;
                    properties.depends = Some(self.parse_expr()?);
                }
                Token::Select => {
                    self.advance()?;
                    let sym = self.parse_identifier()?;
                    let cond = if matches!(self.current_context().current_token, Token::If) {
                        self.advance()?;
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    properties.select.push((sym, cond));
                }
                Token::Imply => {
                    self.advance()?;
                    let sym = self.parse_identifier()?;
                    let cond = if matches!(self.current_context().current_token, Token::If) {
                        self.advance()?;
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    properties.imply.push((sym, cond));
                }
                Token::Range => {
                    self.advance()?;
                    let min = self.parse_expr()?;
                    let max = self.parse_expr()?;
                    let cond = if matches!(self.current_context().current_token, Token::If) {
                        self.advance()?;
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    properties.range = Some((min, max, cond));
                }
                Token::Help => {
                    // Don't advance yet - skip help text directly from lexer
                    let ctx = self.current_context_mut();
                    let help_text = ctx.lexer.skip_help_text();
                    properties.help = Some(help_text);
                    // Now get the next token after skipping help
                    ctx.current_token = ctx.lexer.next_token()?;
                }
                Token::Newline => {
                    self.advance()?;
                }
                _ => break,
            }
        }

        Ok((symbol_type, properties))
    }

    fn parse_choice(&mut self) -> Result<Choice> {
        self.advance()?; // consume 'choice'
        self.skip_newlines()?;

        let name = None;
        let mut prompt = None;
        let mut symbol_type = SymbolType::Bool;
        let mut default = None;
        let mut depends = None;
        let mut options = Vec::new();

        // Parse choice options
        while !matches!(self.current_context().current_token, Token::EndChoice) {
            match &self.current_context().current_token.clone() {
                Token::Prompt => {
                    self.advance()?;
                    prompt = Some(self.parse_string()?);
                }
                Token::Bool => {
                    self.advance()?;
                    symbol_type = SymbolType::Bool;
                }
                Token::Tristate => {
                    self.advance()?;
                    symbol_type = SymbolType::Tristate;
                }
                Token::Default => {
                    self.advance()?;
                    default = Some(self.parse_identifier()?);
                }
                Token::Depends => {
                    self.advance()?;
                    self.expect(Token::On)?;
                    depends = Some(self.parse_expr()?);
                }
                Token::Config => {
                    options.push(self.parse_config()?);
                }
                Token::Newline => {
                    self.advance()?;
                }
                _ => break,
            }
        }

        self.expect(Token::EndChoice)?;
        self.skip_newlines()?;

        Ok(Choice {
            name,
            prompt,
            symbol_type,
            default,
            depends,
            options,
        })
    }

    fn parse_menu(&mut self) -> Result<Menu> {
        self.advance()?; // consume 'menu'
        let title = self.parse_string()?;
        self.skip_newlines()?;

        let mut depends = None;
        let mut visible = None;

        // Parse menu attributes
        while matches!(
            self.current_context().current_token,
            Token::Depends | Token::Visible
        ) {
            match &self.current_context().current_token {
                Token::Depends => {
                    self.advance()?;
                    self.expect(Token::On)?;
                    depends = Some(self.parse_expr()?);
                    self.skip_newlines()?;
                }
                Token::Visible => {
                    self.advance()?;
                    self.expect(Token::If)?;
                    visible = Some(self.parse_expr()?);
                    self.skip_newlines()?;
                }
                _ => break,
            }
        }

        let entries = self.parse_entries()?;

        self.expect(Token::EndMenu)?;
        self.skip_newlines()?;

        Ok(Menu {
            title,
            depends,
            visible,
            entries,
        })
    }

    fn parse_if(&mut self) -> Result<If> {
        self.advance()?; // consume 'if'
        let condition = self.parse_expr()?;
        self.skip_newlines()?;

        let entries = self.parse_entries()?;

        self.expect(Token::EndIf)?;
        self.skip_newlines()?;

        Ok(If { condition, entries })
    }

    fn parse_comment(&mut self) -> Result<Comment> {
        self.advance()?; // consume 'comment'
        let text = self.parse_string()?;
        self.skip_newlines()?;

        let mut depends = None;
        if matches!(self.current_context().current_token, Token::Depends) {
            self.advance()?;
            self.expect(Token::On)?;
            depends = Some(self.parse_expr()?);
            self.skip_newlines()?;
        }

        Ok(Comment { text, depends })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_and_expr()?;

        while matches!(self.current_context().current_token, Token::Or) {
            self.advance()?;
            let right = self.parse_and_expr()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison_expr()?;

        while matches!(self.current_context().current_token, Token::And) {
            self.advance()?;
            let right = self.parse_comparison_expr()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr> {
        let left = self.parse_unary_expr()?;

        match &self.current_context().current_token {
            Token::Eq => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::Equal(Box::new(left), Box::new(right)))
            }
            Token::NotEq => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::NotEqual(Box::new(left), Box::new(right)))
            }
            Token::Less => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::Less(Box::new(left), Box::new(right)))
            }
            Token::LessEq => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::LessEqual(Box::new(left), Box::new(right)))
            }
            Token::Greater => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::Greater(Box::new(left), Box::new(right)))
            }
            Token::GreaterEq => {
                self.advance()?;
                let right = self.parse_unary_expr()?;
                Ok(Expr::GreaterEqual(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    fn parse_unary_expr(&mut self) -> Result<Expr> {
        if matches!(self.current_context().current_token, Token::Not) {
            self.advance()?;
            let expr = self.parse_unary_expr()?;
            return Ok(Expr::Not(Box::new(expr)));
        }

        self.parse_primary_expr()
    }

    fn parse_primary_expr(&mut self) -> Result<Expr> {
        match &self.current_context().current_token.clone() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                Ok(Expr::Symbol(name))
            }
            Token::StringLit(val) => {
                let val = val.clone();
                self.advance()?;
                Ok(Expr::Const(val))
            }
            Token::Number(n) => {
                let n = *n;
                self.advance()?;
                Ok(Expr::Const(n.to_string()))
            }
            Token::LParen => {
                self.advance()?;
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(KconfigError::Syntax {
                file: self.current_file.clone(),
                line: self.current_context().lexer.current_line(),
                message: format!(
                    "Expected expression, got {:?}",
                    self.current_context().current_token
                ),
            }),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        match &self.current_context().current_token {
            Token::StringLit(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(s)
            }
            Token::Identifier(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(s)
            }
            _ => Err(KconfigError::Syntax {
                file: self.current_file.clone(),
                line: self.current_context().lexer.current_line(),
                message: format!(
                    "Expected string, got {:?}",
                    self.current_context().current_token
                ),
            }),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        match &self.current_context().current_token {
            Token::Identifier(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(s)
            }
            _ => Err(KconfigError::Syntax {
                file: self.current_file.clone(),
                line: self.current_context().lexer.current_line(),
                message: format!(
                    "Expected identifier, got {:?}",
                    self.current_context().current_token
                ),
            }),
        }
    }

    fn try_parse_prompt(&mut self) -> Result<String> {
        if matches!(
            self.current_context().current_token,
            Token::StringLit(_) | Token::Identifier(_)
        ) {
            self.parse_string()
        } else {
            Err(KconfigError::Parse("No prompt found".to_string()))
        }
    }

}
