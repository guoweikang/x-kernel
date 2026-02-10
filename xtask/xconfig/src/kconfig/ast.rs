use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    Bool,
    Tristate,
    String,
    Int,
    Hex,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Symbol(String),
    Const(String),
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

#[derive(Debug, Clone)]
pub struct Property {
    pub prompt: Option<String>,
    pub default: Option<Expr>,
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
            default: None,
            depends: None,
            select: Vec::new(),
            imply: Vec::new(),
            range: None,
            help: None,
        }
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
