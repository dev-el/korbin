use ropey::Rope;
use std::{collections, sync::{LazyLock, Mutex}};

pub mod script;
pub mod highlighter;

use highlighter::{SyntaxHighlighter, HighlightSpan};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BufferType {
    Text,
    Directory,
}

impl BufferType {
    pub fn name(&self) -> &str {
        match self {
            BufferType::Text => "text",
            BufferType::Directory => "directory",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Mode {
    Normal,
    Insert,
    Command,
    Visual,
    Search
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Selection {
    pub start: (usize, usize),
    pub end: (usize, usize),
}

#[derive(Clone, Debug)]
pub enum Action {
    EditorCommand(String),
    ShellCommand { cmd: String, async_exec: bool },
}

#[derive(Clone, Default)]
pub struct KeybindingRegistry {
    pub bindings: collections::hash_map::HashMap<(Mode, String, String), Action>,
}

impl KeybindingRegistry {
    fn normalize_key_sequence(seq: &str) -> String {
        let mut normalized = String::new();
        let mut i = 0;
        let chars: Vec<char> = seq.chars().collect();
        while i < chars.len() {
            if chars[i] == '<' {
                let mut j = i;
                while j < chars.len() && chars[j] != '>' { j += 1; }
                if j < chars.len() {
                    let token = &seq[i..=j].to_lowercase();
                    if token == "<space>" { normalized.push(' '); i = j + 1; continue; }
                    if token == "<cr>" || token == "<enter>" { normalized.push('\n'); i = j + 1; continue; }
                    if token == "<esc>" { normalized.push('\x1b'); i = j + 1; continue; }
                    if token == "<tab>" { normalized.push('\t'); i = j + 1; continue; }
                }
            }
            if chars[i] != ' ' {
                normalized.push(chars[i]);
            }
            i += 1;
        }
        if normalized.is_empty() && seq.contains(' ') {
            normalized = " ".to_string();
        }
        normalized
    }
    pub fn set(&mut self, mode: &str, context: &str, key: &str, action: Action) {
        let mode = match mode {
            "normal" => Mode::Normal,
            "insert" => Mode::Insert,
            "command" => Mode::Command,
            "visual" => Mode::Visual,
            _ => return,
        };
        let normalized = Self::normalize_key_sequence(key);
        self.bindings.insert((mode, context.to_string(), normalized), action);
    }

    pub fn get_action(&self, mode: Mode, buffer_type: &BufferType, file_type: &str, key: &str) -> Option<Action> {
        if let Some(action) = self.find_binding(mode, file_type, key) {
            return Some(action);
        }
        
        if let Some(action) = self.find_binding(mode, buffer_type.name(), key) {
            return Some(action);
        }
        
        if let Some(action) = self.find_binding(mode, "global", key) {
            return Some(action);
        }
        
        None
    }

    pub fn is_prefix(&self, mode: Mode, buffer_type: &BufferType, file_type: &str, key: &str) -> bool {
        let contexts = ["global", buffer_type.name(), file_type];
        for ctx in contexts {
          if self.has_prefix(mode, ctx, key) { return true; }
        }
        false
    }

    fn has_prefix(&self, mode: Mode, context: &str, key: &str) -> bool {
        self.bindings.keys()
            .any(|(m, c, k)| *m == mode && c == context && k.starts_with(key) && k != key)
    }

    fn find_binding(&self, mode: Mode, context: &str, key: &str) -> Option<Action> {
        self.bindings.get(&(mode, context.to_string(), key.to_string())).cloned()
    }


}

pub static REGISTRY: LazyLock<Mutex<KeybindingRegistry>> = LazyLock::new(|| {
    let mut r = KeybindingRegistry::default();
    // Default bindings
    r.set("normal", "global", "j", Action::EditorCommand("move_left".into()));
    r.set("normal", "global", "l", Action::EditorCommand("move_right".into()));
    r.set("normal", "global", "k", Action::EditorCommand("move_down".into()));
    r.set("normal", "global", "i", Action::EditorCommand("move_up".into()));
    r.set("normal", "global", "n", Action::EditorCommand("enter_insert".into()));
    r.set("normal", "global", "v", Action::EditorCommand("enter_visual".into()));
    r.set("normal", "global", ":", Action::EditorCommand("enter_command".into()));
    r.set("normal", "global", "fj", Action::EditorCommand("enter_search_forward".into()));
    r.set("normal", "global", "fJ", Action::EditorCommand("enter_search_backward".into()));
    r.set("normal", "global", "fk", Action::EditorCommand("search_next".into()));
    r.set("normal", "global", "fi", Action::EditorCommand("search_prev".into()));
    r.set("normal", "global", "x", Action::EditorCommand("delete_char_at_cursor".into()));
    r.set("normal", "global", "d", Action::EditorCommand("delete_operator".into()));
    r.set("normal", "global", "y", Action::EditorCommand("yank_operator".into()));
    r.set("normal", "global", "p", Action::EditorCommand("paste_after".into()));
    r.set("normal", "global", "P", Action::EditorCommand("paste_before".into()));
    r.set("normal", "global", "u", Action::EditorCommand("undo".into()));
    r.set("normal", "global", "h", Action::EditorCommand("redo".into()));
    
    // Visual mode bindings
    r.set("visual", "global", "j", Action::EditorCommand("move_left".into()));
    r.set("visual", "global", "l", Action::EditorCommand("move_right".into()));
    r.set("visual", "global", "k", Action::EditorCommand("move_down".into()));
    r.set("visual", "global", "i", Action::EditorCommand("move_up".into()));
    r.set("visual", "global", "x", Action::EditorCommand("delete_selection".into()));
    r.set("visual", "global", "d", Action::EditorCommand("delete_selection".into()));
    r.set("visual", "global", "y", Action::EditorCommand("yank_selection".into()));
    r.set("visual", "global", "p", Action::EditorCommand("paste_after".into()));
    Mutex::new(r)
});

pub struct Editor {
    pub buffer_type: BufferType,
    pub file_type: String,
    pub mode: Mode,
    pub text: Rope,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub selection: Option<Selection>,
    pub command_buffer: String,
    pub key_buffer: String,
    pub last_search: String,
    pub search_forward: bool,
    pub pending_operator: Option<char>,
    pub file_path: Option<String>,
    pub status_message: Option<String>,
    pub version: usize,
    pub highlighter: std::cell::RefCell<SyntaxHighlighter>,
    pub registers: std::collections::HashMap<char, (String, bool)>,
    pub selected_register: char,
    pub waiting_for_register_name: bool,
    pub history: Vec<(Rope, (usize, usize), (usize, usize))>,
    pub history_index: usize,
    pub cursor_before_edit: (usize, usize),
}

impl Editor {
    pub fn new() -> Self {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language_from_path("welcome.md");
        let text = Rope::from_str("# Welcome to Korbin!\n\nA scriptable text editor with native performance.\n\n## Features\n- Vi-like Keybindings\n- Embedded Scripting\n- Ribir-based UI\n- Tree-sitter Support\n- Rope Text Structure\n");
        let mut history = Vec::new();
        history.push((text.clone(), (0, 0), (0, 0)));
        Self {
            buffer_type: BufferType::Text,
            file_type: "markdown".to_string(),
            mode: Mode::Normal,
            text,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            command_buffer: String::new(),
            key_buffer: String::new(),
            last_search: String::new(),
            search_forward: true,
            pending_operator: None,
            file_path: None,
            status_message: None,
            version: 0,
            highlighter: std::cell::RefCell::new(highlighter),
            registers: std::collections::HashMap::new(),
            selected_register: '"',
            waiting_for_register_name: false,
            history,
            history_index: 0,
            cursor_before_edit: (0, 0),
        }
    }

    fn update_file_type_from_path(&mut self, path: &str) {
        if path.ends_with(".rs") {
            self.file_type = "rust".to_string();
        } else if path.ends_with(".md") {
            self.file_type = "markdown".to_string();
        } else if path.ends_with(".tex") {
            self.file_type = "latex".to_string();
        } else {
            self.file_type = "text".to_string();
        }
    }

    fn mark_changed(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    pub fn begin_edit(&mut self) {
        self.cursor_before_edit = (self.cursor_line, self.cursor_col);
    }

    pub fn push_history(&mut self) {
        let current_text = self.text.clone();
        if let Some((last_text, _, _)) = self.history.last() {
            if last_text == &current_text {
                return; // No change
            }
        }
        
        // Truncate history if we are not at the end
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        
        self.history.push((
            current_text,
            self.cursor_before_edit,
            (self.cursor_line, self.cursor_col)
        ));
        self.history_index = self.history.len() - 1;
    }

    pub fn undo(&mut self) {
        if self.history_index > 0 {
            let (_, cursor_before, _) = self.history[self.history_index].clone();
            self.history_index -= 1;
            let (text, _, _) = self.history[self.history_index].clone();
            
            self.text = text;
            self.cursor_line = cursor_before.0;
            self.cursor_col = cursor_before.1;
            self.mark_changed();
            self.status_message = Some("Undo".to_string());
        } else {
            self.status_message = Some("Already at oldest change".to_string());
        }
    }

    pub fn redo(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let (text, _, cursor_after) = self.history[self.history_index].clone();
            self.text = text;
            self.cursor_line = cursor_after.0;
            self.cursor_col = cursor_after.1;
            self.mark_changed();
            self.status_message = Some("Redo".to_string());
        } else {
            self.status_message = Some("Already at newest change".to_string());
        }
    }

    fn set_register(&mut self, text: String, is_linewise: bool, is_yank: bool) {
        self.registers.insert(self.selected_register, (text.clone(), is_linewise));
        if is_yank && self.selected_register == '"' {
            self.registers.insert('0', (text, is_linewise));
        }
        self.selected_register = '"';
    }

    pub fn move_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.clamp_cursor();
        }
        self.update_selection();
        self.mark_changed();
    }

    pub fn move_down(&mut self) {
        if self.cursor_line + 1 < self.text.len_lines() {
            self.cursor_line += 1;
            self.clamp_cursor();
        }
        self.update_selection();
        self.mark_changed();
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
        self.update_selection();
        self.mark_changed();
    }

    pub fn move_right(&mut self) {
        let line = self.text.line(self.cursor_line);
        let max_col = line.len_chars().saturating_sub(1);
        if self.cursor_col < max_col {
            self.cursor_col += 1;
        }
        self.update_selection();
        self.mark_changed();
    }

    fn clamp_cursor(&mut self) {
        let line = self.text.line(self.cursor_line);
        let max_col = line.len_chars().saturating_sub(1);
        if self.cursor_col > max_col {
            self.cursor_col = max_col;
        }
    }

    pub fn get_cursor_pos(&self) -> usize {
        self.text.line_to_char(self.cursor_line) + self.cursor_col
    }

    pub fn insert_char(&mut self, c: char) {
        let pos = self.get_cursor_pos();
        self.text.insert_char(pos, c);
        if c == '\n' {
            self.cursor_line += 1;
            self.cursor_col = 0;
        } else {
            self.cursor_col += 1;
        }
        self.mark_changed();
    }

    pub fn delete_char(&mut self) {
        let pos = self.get_cursor_pos();
        if pos > 0 {
            self.text.remove(pos - 1..pos);
            let (line, col) = self.char_to_line_col(pos - 1);
            self.cursor_line = line;
            self.cursor_col = col;
        }
        self.mark_changed();
    }

    pub fn delete_char_at_cursor(&mut self) {
        let pos = self.get_cursor_pos();
        if pos < self.text.len_chars() {
            let line = self.text.line(self.cursor_line);
            if self.cursor_col < line.len_chars().saturating_sub(1) || (self.cursor_line + 1 == self.text.len_lines() && self.cursor_col < line.len_chars()) {
                let deleted_text = self.text.slice(pos..pos + 1).to_string();
                self.text.remove(pos..pos + 1);
                self.set_register(deleted_text, false, false);
                self.clamp_cursor();
            }
        }
        self.mark_changed();
    }

    pub fn delete_current_line(&mut self) {
        if self.text.len_lines() > 0 {
            let start = self.text.line_to_char(self.cursor_line);
            let end = if self.cursor_line + 1 < self.text.len_lines() {
                self.text.line_to_char(self.cursor_line + 1)
            } else {
                self.text.len_chars()
            };
            let deleted_text = self.text.slice(start..end).to_string();
            self.text.remove(start..end);
            self.set_register(deleted_text, true, false);
            if self.cursor_line >= self.text.len_lines() && self.cursor_line > 0 {
                self.cursor_line -= 1;
            }
            self.clamp_cursor();
        }
        self.mark_changed();
    }

    fn char_to_line_col(&self, pos: usize) -> (usize, usize) {
        let line = self.text.char_to_line(pos.min(self.text.len_chars()));
        let line_start = self.text.line_to_char(line);
        (line, pos - line_start)
    }

    pub fn enter_visual(&mut self) {
        self.mode = Mode::Visual;
        self.selection = Some(Selection {
            start: (self.cursor_line, self.cursor_col),
            end: (self.cursor_line, self.cursor_col),
        });
        self.mark_changed();
    }

    pub fn exit_visual(&mut self) {
        self.mode = Mode::Normal;
        self.selection = None;
        self.mark_changed();
    }

    pub fn update_selection(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end = (self.cursor_line, self.cursor_col);
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some(sel) = self.selection {
            let start_pos = self.text.line_to_char(sel.start.0) + sel.start.1;
            let end_pos = self.text.line_to_char(sel.end.0) + sel.end.1;
            
            let (s, e) = if start_pos <= end_pos {
                (start_pos, end_pos + 1)
            } else {
                (end_pos, start_pos + 1)
            };

            let safe_e = e.min(self.text.len_chars());
            if s < safe_e {
                let deleted_text = self.text.slice(s..safe_e).to_string();
                self.text.remove(s..safe_e);
                self.set_register(deleted_text, false, false);
                let (line, col) = self.char_to_line_col(s);
                self.cursor_line = line;
                self.cursor_col = col;
            }
            self.exit_visual();
            self.clamp_cursor();
            self.mark_changed();
        }
    }

    pub fn yank_current_line(&mut self) {
        if self.cursor_line < self.text.len_lines() {
            let start = self.text.line_to_char(self.cursor_line);
            let end = if self.cursor_line + 1 < self.text.len_lines() {
                self.text.line_to_char(self.cursor_line + 1)
            } else {
                self.text.len_chars()
            };
            let text = self.text.slice(start..end).to_string();
            self.set_register(text, true, true);
            self.status_message = Some("1 line yanked".to_string());
        }
    }

    pub fn yank_selection(&mut self) {
        if let Some(sel) = self.selection {
            let start_pos = self.text.line_to_char(sel.start.0) + sel.start.1;
            let end_pos = self.text.line_to_char(sel.end.0) + sel.end.1;
            
            let (s, e) = if start_pos <= end_pos {
                (start_pos, end_pos + 1)
            } else {
                (end_pos, start_pos + 1)
            };

            let safe_e = e.min(self.text.len_chars());
            if s < safe_e {
                let text = self.text.slice(s..safe_e).to_string();
                self.set_register(text, false, true);
                self.status_message = Some(format!("{} characters yanked", safe_e - s));
            }
            self.exit_visual();
        }
    }

    pub fn paste(&mut self, after: bool) {
        if let Some((text, is_linewise)) = self.registers.get(&self.selected_register).cloned() {
            if text.is_empty() { return; }
            
            if is_linewise {
                let insert_line = if after { self.cursor_line + 1 } else { self.cursor_line };
                let pos = if insert_line >= self.text.len_lines() { self.text.len_chars() } else { self.text.line_to_char(insert_line) };
                let mut text_to_insert = text.clone();
                if !text_to_insert.ends_with('\n') { text_to_insert.push('\n'); }
                self.text.insert(pos, &text_to_insert);
                self.cursor_line = insert_line;
                self.cursor_col = 0;
            } else {
                let pos = self.get_cursor_pos();
                let insert_pos = if after && pos < self.text.len_chars() {
                    let line_text = self.text.line(self.cursor_line);
                    let line_len = line_text.len_chars().saturating_sub(1);
                    if self.cursor_col < line_len { pos + 1 } else { pos }
                } else { pos };
                self.text.insert(insert_pos, &text);
                let (line, col) = self.char_to_line_col(insert_pos + text.chars().count().saturating_sub(1));
                self.cursor_line = line;
                self.cursor_col = col;
            }
            self.mark_changed();
        }
        self.selected_register = '"';
    }

    pub fn paste_replace_selection(&mut self) {
        if let Some((text, _)) = self.registers.get(&self.selected_register).cloned() {
            if text.is_empty() { return; }
            if let Some(sel) = self.selection {
                let start_pos = self.text.line_to_char(sel.start.0) + sel.start.1;
                let end_pos = self.text.line_to_char(sel.end.0) + sel.end.1;
                let (s, e) = if start_pos <= end_pos { (start_pos, end_pos + 1) } else { (end_pos, start_pos + 1) };
                let safe_e = e.min(self.text.len_chars());
                if s < safe_e {
                    self.text.remove(s..safe_e);
                    self.text.insert(s, &text);
                    let (line, col) = self.char_to_line_col(s + text.chars().count().saturating_sub(1));
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                self.exit_visual();
                self.clamp_cursor();
                self.mark_changed();
            }
        }
        self.selected_register = '"';
    }

    pub fn perform_search(&mut self, query: &str, forward: bool) {
        if query.is_empty() { return; }
        let pos = self.get_cursor_pos();
        let text_str = self.text.to_string();
        let found = if forward {
            text_str[pos + 1..].find(query).map(|i| i + pos + 1)
        } else {
            text_str[..pos].rfind(query)
        };

        if let Some(new_pos) = found {
            let (line, col) = self.char_to_line_col(new_pos);
            self.cursor_line = line;
            self.cursor_col = col;
            self.status_message = None;
        } else {
            self.status_message = Some(format!("Pattern not found: {}", query));
        }
        self.mark_changed();
    }

    pub fn display_text(&self) -> String {
        self.text.to_string().replace('\t', "    ")
    }

    pub fn rope_byte_to_display_byte(&self, byte_pos: usize) -> usize {
        if byte_pos == 0 {
            return 0;
        }
        let safe_byte_pos = byte_pos.min(self.text.len_bytes());
        let prefix = self.text.byte_slice(..safe_byte_pos);
        let mut tab_count = 0;
        for chunk in prefix.chunks() {
            tab_count += chunk.as_bytes().iter().filter(|&&b| b == b'\t').count();
        }
        safe_byte_pos + tab_count * 3
    }

    pub fn get_selection_char_bounds(&self) -> Option<(usize, usize)> {
        self.selection.map(|sel| {
            let start_pos = self.text.line_to_char(sel.start.0) + sel.start.1;
            let end_pos = self.text.line_to_char(sel.end.0) + sel.end.1;
            if start_pos <= end_pos { (start_pos, end_pos + 1) } else { (end_pos, start_pos + 1) }
        })
    }

    pub fn get_highlight_spans(&self) -> Vec<HighlightSpan> {
        self.highlighter.borrow_mut().highlight(&self.text.to_string())
    }

    pub fn get_display_highlight_spans(&self) -> Vec<HighlightSpan> {
        self.highlighter.borrow_mut().highlight(&self.display_text())
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let file = std::fs::File::create(path)?;
            self.text.write_to(file)?;
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "No file path set"))
        }
    }

    pub fn save_as(&mut self, path: String) -> std::io::Result<()> {
        let file = std::fs::File::create(&path)?;
        self.text.write_to(file)?;
        self.highlighter.borrow_mut().set_language_from_path(&path);
        self.update_file_type_from_path(&path);
        self.file_path = Some(path);
        self.mark_changed();
        Ok(())
    }

    pub fn open(&mut self, path: String) -> std::io::Result<()> {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if metadata.is_dir() { return self.open_dired(path); }
        }
        let file = std::fs::File::open(&path)?;
        let new_text = Rope::from_reader(file)?;
        self.text = new_text;
        self.highlighter.borrow_mut().set_language_from_path(&path);
        self.update_file_type_from_path(&path);
        self.file_path = Some(path);
        self.buffer_type = BufferType::Text;
        self.mode = Mode::Normal;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = None;
        self.mark_changed();
        Ok(())
    }

    pub fn open_dired(&mut self, path: String) -> std::io::Result<()> {
        let mut entries = Vec::new();
        if let Ok(parent) = std::path::Path::new(&path).canonicalize() {
            if let Some(_p) = parent.parent() { entries.push(("..".to_string(), true)); }
        }
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type()?.is_dir();
            entries.push((name, is_dir));
        }
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let mut dired_text = String::new();
        dired_text.push_str(&format!("// Directory: {}\n", std::fs::canonicalize(&path).unwrap_or(std::path::PathBuf::from(&path)).to_string_lossy()));
        for (name, is_dir) in entries {
            if is_dir { dired_text.push_str(&format!("  {}/\n", name)); }
            else { dired_text.push_str(&format!("  {}\n", name)); }
        }
        self.text = Rope::from_str(&dired_text);
        self.highlighter.borrow_mut().set_language_from_path("dired.md");
        self.file_type = "directory".to_string();
        self.file_path = Some(path);
        self.buffer_type = BufferType::Directory;
        self.mode = Mode::Normal;
        self.cursor_line = 1;
        self.cursor_col = 2;
        self.selection = None;
        self.mark_changed();
        Ok(())
    }

    pub fn trigger_action(&mut self) {
        if self.buffer_type == BufferType::Directory {
            if self.cursor_line < self.text.len_lines() {
                let line = self.text.line(self.cursor_line).to_string();
                let entry_name = line.trim();
                if entry_name.is_empty() || entry_name.starts_with("//") { return; }
                let entry_name = entry_name.trim_end_matches('/');
                if let Some(ref current_path) = self.file_path {
                    let mut path = std::path::PathBuf::from(current_path);
                    if entry_name == ".." { path = path.parent().unwrap_or(&path).to_path_buf(); }
                    else { path.push(entry_name); }
                    if let Ok(clean_path) = std::fs::canonicalize(&path) { let _ = self.open(clean_path.to_string_lossy().to_string()); }
                    else { let _ = self.open(path.to_string_lossy().to_string()); }
                }
            }
        }
    }

    pub fn dispatch_command(&mut self, command: &str) {
        match command {
            "move_up" => self.move_up(),
            "move_down" => self.move_down(),
            "move_left" => self.move_left(),
            "move_right" => self.move_right(),
            "save" => {
                let _ = self.save();
            }
            "quit" => {
                std::process::exit(0);
            }
            "enter_insert" => if self.buffer_type == BufferType::Text { self.begin_edit(); self.mode = Mode::Insert; self.pending_operator = None; self.status_message = None; }
            "enter_visual" => if self.buffer_type == BufferType::Text { self.enter_visual(); self.pending_operator = None; self.status_message = None; }
            "enter_command" => { self.mode = Mode::Command; self.command_buffer.clear(); self.pending_operator = None; self.status_message = None; }
            "enter_search_forward" => {
                self.mode = Mode::Search;
                self.command_buffer.clear();
                self.last_search.clear();
                self.search_forward = true;
                self.pending_operator = None;
                self.status_message = None;
            }
            "enter_search_backward" => {
                self.mode = Mode::Search;
                self.command_buffer.clear();
                self.last_search.clear();
                self.search_forward = false;
                self.pending_operator = None;
                self.status_message = None;
            }
            "search_next" => {
                let query = self.last_search.clone();
                let forward = self.search_forward;
                self.perform_search(&query, forward);
                self.pending_operator = None;
            }
            "search_prev" => {
                let query = self.last_search.clone();
                let forward = !self.search_forward;
                self.perform_search(&query, forward);
                self.pending_operator = None;
            }
            "delete_char_at_cursor" => { if self.buffer_type == BufferType::Text { self.begin_edit(); self.delete_char_at_cursor(); self.push_history(); } self.pending_operator = None; }
            "delete_selection" => { if self.buffer_type == BufferType::Text { self.begin_edit(); self.delete_selection(); self.push_history(); } self.pending_operator = None; }
            "delete_operator" => if self.buffer_type == BufferType::Text { if self.pending_operator == Some('d') { self.begin_edit(); self.delete_current_line(); self.push_history(); self.pending_operator = None; } else { self.pending_operator = Some('d'); } } else { self.pending_operator = None; }
            "yank_operator" => if self.buffer_type == BufferType::Text { if self.pending_operator == Some('y') { self.yank_current_line(); self.pending_operator = None; } else { self.pending_operator = Some('y'); } } else { self.pending_operator = None; }
            "yank_selection" => { if self.buffer_type == BufferType::Text { self.yank_selection(); } self.pending_operator = None; }
            "paste_after" => { if self.buffer_type == BufferType::Text { if self.mode == Mode::Visual { self.paste_replace_selection(); } else { self.paste(true); } } self.pending_operator = None; }
            "paste_before" => { if self.buffer_type == BufferType::Text { if self.mode == Mode::Visual { self.paste_replace_selection(); } else { self.paste(false); } } self.pending_operator = None; }
            "undo" => {
                if self.buffer_type == BufferType::Text {
                    self.undo();
                }
                self.pending_operator = None;
            }
            "redo" => {
                if self.buffer_type == BufferType::Text {
                    self.redo();
                }
                self.pending_operator = None;
            }
            _ => {}
        }
        self.mark_changed();
    }

    pub fn get_selection_ranges(&self) -> Vec<(usize, usize, usize)> {
        let mut ranges = Vec::new();
        if let Some(sel) = self.selection {
            let (s_l, s_c, e_l, e_c) = if (sel.start.0, sel.start.1) <= (sel.end.0, sel.end.1) { (sel.start.0, sel.start.1, sel.end.0, sel.end.1) }
            else { (sel.end.0, sel.end.1, sel.start.0, sel.start.1) };
            for line_idx in s_l..=e_l {
                let line_text = self.text.line(line_idx);
                let is_last_line = line_idx + 1 == self.text.len_lines();
                let line_len = if is_last_line { line_text.len_chars() } else { line_text.len_chars().saturating_sub(1) };
                let col_start = if line_idx == s_l { s_c } else { 0 };
                let col_end = if line_idx == e_l { e_c } else { line_len.saturating_sub(1) };
                if col_start <= col_end { ranges.push((line_idx, col_start, col_end)); }
            }
        }
        ranges
    }
}
