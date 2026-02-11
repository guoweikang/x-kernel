use crate::kconfig::ast::{Entry, Menu, Config, MenuConfig, Choice, Comment};
use crate::kconfig::{SymbolType, Expr};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub kind: MenuItemKind,
    pub label: String,
    pub value: Option<ConfigValue>,
    pub is_visible: bool,
    pub is_enabled: bool,
    pub has_children: bool,
    pub depth: usize,
    pub help_text: Option<String>,
    pub depends_on: Option<Expr>,
    pub selects: Vec<String>,
    pub implies: Vec<String>,
    pub selected_by: Vec<String>,
    pub implied_by: Vec<String>,
    pub parent_choice: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MenuItemKind {
    Menu { title: String },
    Config { symbol_type: SymbolType },
    MenuConfig { symbol_type: SymbolType },
    Choice { options: Vec<String> },
    Comment { text: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Tristate(TristateValue),
    String(String),
    Int(i64),
    Hex(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TristateValue {
    Yes,
    No,
    Module,
}

impl MenuItem {
    pub fn from_config(config: &Config, depth: usize) -> Self {
        Self {
            id: config.name.clone(),
            kind: MenuItemKind::Config {
                symbol_type: config.symbol_type.clone(),
            },
            label: config.properties.prompt.clone().unwrap_or_else(|| config.name.clone()),
            value: None,
            is_visible: true,
            is_enabled: true,
            has_children: false,
            depth,
            help_text: config.properties.help.clone(),
            depends_on: config.properties.depends.clone(),
            selects: config.properties.select.iter().map(|(s, _)| s.clone()).collect(),
            implies: config.properties.imply.iter().map(|(s, _)| s.clone()).collect(),
            selected_by: Vec::new(),
            implied_by: Vec::new(),
            parent_choice: None,
        }
    }
    
    pub fn from_menuconfig(config: &MenuConfig, depth: usize) -> Self {
        Self {
            id: config.name.clone(),
            kind: MenuItemKind::MenuConfig {
                symbol_type: config.symbol_type.clone(),
            },
            label: config.properties.prompt.clone().unwrap_or_else(|| config.name.clone()),
            value: None,
            is_visible: true,
            is_enabled: true,
            has_children: true,
            depth,
            help_text: config.properties.help.clone(),
            depends_on: config.properties.depends.clone(),
            selects: config.properties.select.iter().map(|(s, _)| s.clone()).collect(),
            implies: config.properties.imply.iter().map(|(s, _)| s.clone()).collect(),
            selected_by: Vec::new(),
            implied_by: Vec::new(),
            parent_choice: None,
        }
    }
    
    pub fn from_menu(menu: &Menu, depth: usize) -> Self {
        Self {
            id: format!("menu_{}", menu.title),
            kind: MenuItemKind::Menu {
                title: menu.title.clone(),
            },
            label: menu.title.clone(),
            value: None,
            is_visible: true,
            is_enabled: true,
            has_children: true,
            depth,
            help_text: None,
            depends_on: menu.depends.clone(),
            selects: Vec::new(),
            implies: Vec::new(),
            selected_by: Vec::new(),
            implied_by: Vec::new(),
            parent_choice: None,
        }
    }
    
    pub fn from_choice(choice: &Choice, depth: usize) -> Self {
        let options: Vec<String> = choice.options.iter().map(|c| c.name.clone()).collect();
        Self {
            id: choice.name.clone().unwrap_or_else(|| "choice".to_string()),
            kind: MenuItemKind::Choice {
                options: options.clone(),
            },
            label: choice.prompt.clone().unwrap_or_else(|| "Choice".to_string()),
            value: None,
            is_visible: true,
            is_enabled: true,
            has_children: !options.is_empty(),
            depth,
            help_text: None,
            depends_on: choice.depends.clone(),
            selects: Vec::new(),
            implies: Vec::new(),
            selected_by: Vec::new(),
            implied_by: Vec::new(),
            parent_choice: None,
        }
    }
    
    pub fn from_comment(comment: &Comment, depth: usize) -> Self {
        Self {
            id: format!("comment_{}", comment.text),
            kind: MenuItemKind::Comment {
                text: comment.text.clone(),
            },
            label: comment.text.clone(),
            value: None,
            is_visible: true,
            is_enabled: true,
            has_children: false,
            depth,
            help_text: None,
            depends_on: comment.depends.clone(),
            selects: Vec::new(),
            implies: Vec::new(),
            selected_by: Vec::new(),
            implied_by: Vec::new(),
            parent_choice: None,
        }
    }
}

pub struct NavigationState {
    pub current_path: Vec<String>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            current_path: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }
}

impl Default for NavigationState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConfigState {
    pub all_items: Vec<MenuItem>,
    pub menu_tree: HashMap<String, Vec<MenuItem>>,
    pub modified_symbols: HashMap<String, String>,
    pub original_values: HashMap<String, String>,
}

impl ConfigState {
    pub fn new() -> Self {
        Self {
            all_items: Vec::new(),
            menu_tree: HashMap::new(),
            modified_symbols: HashMap::new(),
            original_values: HashMap::new(),
        }
    }
    
    pub fn build_from_entries(entries: &[Entry]) -> Self {
        let mut state = Self::new();
        state.process_entries(entries, 0, "root");
        state.build_reverse_dependencies();
        state
    }
    
    fn process_entries(&mut self, entries: &[Entry], depth: usize, parent_id: &str) {
        let mut items = Vec::new();
        
        // Process entries and collect them into items
        self.collect_items(entries, depth, parent_id, &mut items);
        
        self.menu_tree.insert(parent_id.to_string(), items.clone());
        self.all_items.extend(items);
    }
    
    /// Recursively collects menu items from entries and appends them to the provided items vector.
    /// 
    /// This helper function is used to handle inline processing of `if` blocks, ensuring that
    /// entries within if blocks are collected into the same items vector as their siblings.
    /// This prevents menu tree overwrites that would occur if if-blocks were processed via
    /// separate calls to `process_entries()`.
    fn collect_items(&mut self, entries: &[Entry], depth: usize, parent_id: &str, items: &mut Vec<MenuItem>) {
        for entry in entries {
            match entry {
                Entry::Config(config) => {
                    let item = MenuItem::from_config(config, depth);
                    items.push(item);
                }
                Entry::MenuConfig(menuconfig) => {
                    let item = MenuItem::from_menuconfig(menuconfig, depth);
                    items.push(item.clone());
                    
                    // MenuConfig can have sub-items (not in this simple version)
                    // In a full implementation, we'd recursively process
                }
                Entry::Menu(menu) => {
                    let item = MenuItem::from_menu(menu, depth);
                    let menu_id = item.id.clone();
                    items.push(item);
                    
                    // Process menu children with new parent_id and depth
                    self.process_entries(&menu.entries, depth + 1, &menu_id);
                }
                Entry::Choice(choice) => {
                    // Generate unique choice ID if not named
                    let choice_id = if let Some(name) = &choice.name {
                        name.clone()
                    } else {
                        // Use the first option name to generate a unique ID
                        if let Some(first_option) = choice.options.first() {
                            format!("choice_{}", first_option.name)
                        } else {
                            "choice_unknown".to_string()
                        }
                    };
                    
                    let mut item = MenuItem::from_choice(choice, depth);
                    item.id = choice_id.clone();
                    items.push(item);
                    
                    // Add choice options as children with parent_choice set
                    for option in &choice.options {
                        let mut opt_item = MenuItem::from_config(option, depth + 1);
                        opt_item.parent_choice = Some(choice_id.clone());
                        items.push(opt_item);
                    }
                }
                Entry::Comment(comment) => {
                    let item = MenuItem::from_comment(comment, depth);
                    items.push(item);
                }
                Entry::If(if_entry) => {
                    // Process if block entries inline - they belong to the same menu level
                    // The if condition is already part of each entry's depends_on field
                    self.collect_items(&if_entry.entries, depth, parent_id, items);
                }
                Entry::MainMenu(_title) => {
                    // Skip mainmenu for now
                }
                Entry::Source(_) => {
                    // Source entries are handled during parsing
                }
            }
        }
    }
    
    pub fn get_items_for_path(&self, path: &[String]) -> Vec<MenuItem> {
        let key = if path.is_empty() {
            "root".to_string()
        } else {
            path.last().unwrap().clone()
        };
        
        self.menu_tree.get(&key).cloned().unwrap_or_default()
    }
    
    fn build_reverse_dependencies(&mut self) {
        // Build maps of reverse dependencies
        let mut selected_by_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut implied_by_map: HashMap<String, Vec<String>> = HashMap::new();
        
        // First pass: collect all reverse dependencies
        for item in &self.all_items {
            for select in &item.selects {
                selected_by_map
                    .entry(select.clone())
                    .or_insert_with(Vec::new)
                    .push(item.id.clone());
            }
            for imply in &item.implies {
                implied_by_map
                    .entry(imply.clone())
                    .or_insert_with(Vec::new)
                    .push(item.id.clone());
            }
        }
        
        // Second pass: update all items with reverse dependencies
        for item in &mut self.all_items {
            if let Some(selected_by) = selected_by_map.get(&item.id) {
                item.selected_by = selected_by.clone();
            }
            if let Some(implied_by) = implied_by_map.get(&item.id) {
                item.implied_by = implied_by.clone();
            }
        }
        
        // Third pass: update menu_tree items with reverse dependencies
        for (_, items) in self.menu_tree.iter_mut() {
            for item in items {
                if let Some(selected_by) = selected_by_map.get(&item.id) {
                    item.selected_by = selected_by.clone();
                }
                if let Some(implied_by) = implied_by_map.get(&item.id) {
                    item.implied_by = implied_by.clone();
                }
            }
        }
    }
}

impl Default for ConfigState {
    fn default() -> Self {
        Self::new()
    }
}
