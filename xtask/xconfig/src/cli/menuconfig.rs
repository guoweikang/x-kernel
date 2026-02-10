use crate::error::Result;
use crate::kconfig::Parser;
use crate::config::ConfigReader;
use crate::ui::MenuConfigApp;
use std::path::PathBuf;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

pub fn menuconfig_command(kconfig: PathBuf, srctree: PathBuf) -> Result<()> {
    println!("Loading configuration...");
    
    // Parse Kconfig
    let mut parser = Parser::new(&kconfig, &srctree)?;
    let ast = parser.parse()?;
    
    println!("Parsed {} entries", ast.entries.len());
    
    // Load existing config if present
    let mut symbol_table = crate::kconfig::SymbolTable::new();
    
    // Extract all symbols from AST
    extract_symbols_from_entries(&ast.entries, &mut symbol_table);
    
    // Load existing .config if it exists
    if std::path::Path::new(".config").exists() {
        println!("Loading existing .config...");
        let config_values = ConfigReader::read(".config")?;
        for (name, value) in config_values {
            symbol_table.set_value(&name, value);
        }
    } else {
        println!("No existing .config found, using defaults");
    }
    
    println!("Launching TUI...");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create and run app
    let mut app = MenuConfigApp::new(ast.entries, symbol_table)?;
    let res = app.run(&mut terminal);
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    
    res
}

fn extract_symbols_from_entries(entries: &[crate::kconfig::ast::Entry], symbol_table: &mut crate::kconfig::SymbolTable) {
    use crate::kconfig::ast::Entry;
    
    for entry in entries {
        match entry {
            Entry::Config(config) => {
                symbol_table.add_symbol(config.name.clone(), config.symbol_type.clone());
                // Set default if specified
                if let Some(default_expr) = &config.properties.default {
                    if let crate::kconfig::Expr::Const(val) = default_expr {
                        symbol_table.set_value(&config.name, val.clone());
                    }
                }
            }
            Entry::MenuConfig(menuconfig) => {
                symbol_table.add_symbol(menuconfig.name.clone(), menuconfig.symbol_type.clone());
            }
            Entry::Choice(choice) => {
                for option in &choice.options {
                    symbol_table.add_symbol(option.name.clone(), option.symbol_type.clone());
                }
            }
            Entry::Menu(menu) => {
                extract_symbols_from_entries(&menu.entries, symbol_table);
            }
            Entry::If(if_entry) => {
                extract_symbols_from_entries(&if_entry.entries, symbol_table);
            }
            _ => {}
        }
    }
}

