use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    Bool,
    Tristate,
    String,
    Int,
    Hex,
    Range,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Symbol(String),
    Const(String),
    ShellExpr(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Equal(Box<Expr>, Box<Expr>),
    NotEqual(Box<Expr>, Box<Expr>),
    Less(Box<Expr>, Box<Expr>),
    LessEqual(Box<Expr>, Box<Expr>),
    Greater(Box<Expr>, Box<Expr>),
    GreaterEqual(Box<Expr>, Box<Expr>),
}

/// Represents a default value with an optional condition
#[derive(Debug, Clone)]
pub struct DefaultValue {
    pub value: Expr,
    pub condition: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Property {
    pub prompt: Option<String>,
    pub defaults: Vec<DefaultValue>,
    pub depends: Option<Expr>,
    pub select: Vec<(String, Option<Expr>)>,
    pub imply: Vec<(String, Option<Expr>)>,
    pub range: Option<(Expr, Expr, Option<Expr>)>,
    pub help: Option<String>,
}

impl Default for Property {
    fn default() -> Self {
        Self {
            prompt: None,
            defaults: Vec::new(),
            depends: None,
            select: Vec::new(),
            imply: Vec::new(),
            range: None,
            help: None,
        }
    }
}

impl Property {
    /// Evaluate conditional defaults in order and return the first matching value
    pub fn evaluate_default(&self, symbol_table: &crate::kconfig::SymbolTable) -> Option<String> {
        use crate::kconfig::expr::evaluate_expr;
        use crate::kconfig::shell_expr::evaluate_shell_expr;

        for default in &self.defaults {
            // Check condition (if any)
            if let Some(ref condition) = default.condition {
                // Skip this default if condition is not met
                if !matches!(evaluate_expr(condition, symbol_table), Ok(true)) {
                    continue;
                }
            }

            // Evaluate the value expression
            match &default.value {
                Expr::Const(val) => return Some(val.clone()),
                Expr::Symbol(sym) => {
                    if let Some(value) = symbol_table.get_value(sym) {
                        return Some(value);
                    }
                }
                Expr::ShellExpr(shell) => {
                    if let Ok(value) = evaluate_shell_expr(shell, symbol_table) {
                        if !value.is_empty() {
                            return Some(value);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub symbol_type: SymbolType,
    pub properties: Property,
}

#[derive(Debug, Clone)]
pub struct MenuConfig {
    pub name: String,
    pub symbol_type: SymbolType,
    pub properties: Property,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub name: Option<String>,
    pub prompt: Option<String>,
    pub symbol_type: SymbolType,
    pub default: Option<String>,
    pub depends: Option<Expr>,
    pub options: Vec<Config>,
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub depends: Option<Expr>,
    pub visible: Option<Expr>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct If {
    pub condition: Expr,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
    pub depends: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum Entry {
    Config(Config),
    MenuConfig(MenuConfig),
    Choice(Choice),
    Menu(Menu),
    If(If),
    Source(Source),
    Comment(Comment),
    MainMenu(String),
}

#[derive(Debug, Clone)]
pub struct KconfigFile {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}
