//! Global TODO loading for the sidebar footer.

mod ui;

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const TODO_RELATIVE_PATH: &str = ".workmux/todos.json";
const DEFAULT_MAX_VISIBLE: usize = 3;
const DEFAULT_MAX_LINES_PER_ITEM: usize = 2;
const MAX_VISIBLE_LIMIT: usize = 20;
const MAX_LINES_PER_ITEM_LIMIT: usize = 10;

#[derive(Debug, Deserialize)]
struct TodoDocument {
    #[serde(default = "default_max_visible")]
    max_visible: usize,
    #[serde(default = "default_max_lines_per_item")]
    max_lines_per_item: usize,
    #[serde(default)]
    todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize)]
struct TodoItem {
    text: String,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct LoadedTodos {
    items: Vec<String>,
    max_visible: usize,
    max_lines_per_item: usize,
}

/// Cached TODO state shared by every sidebar panel.
#[derive(Debug)]
pub(super) struct TodoPanel {
    path: Option<PathBuf>,
    stamp: Option<FileStamp>,
    pub(super) items: Vec<String>,
    pub(super) max_visible: usize,
    pub(super) max_lines_per_item: usize,
    pub(super) error: Option<String>,
    pub(super) expanded: bool,
}

impl Default for TodoPanel {
    fn default() -> Self {
        Self {
            path: global_todo_path(home::home_dir().as_deref()),
            stamp: None,
            items: Vec::new(),
            max_visible: DEFAULT_MAX_VISIBLE,
            max_lines_per_item: DEFAULT_MAX_LINES_PER_ITEM,
            error: None,
            expanded: true,
        }
    }
}

impl TodoPanel {
    pub(super) fn sync(&mut self, force: bool) {
        let Some(path) = self.path.as_ref() else {
            return;
        };

        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.stamp = None;
                self.reset_content();
                self.error = None;
                return;
            }
            Err(error) => {
                self.stamp = None;
                self.reset_content();
                self.error = Some(format!("TODO read error: {error}"));
                return;
            }
        };

        let next_stamp = FileStamp {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        };
        if !force && self.stamp == Some(next_stamp) {
            return;
        }
        self.stamp = Some(next_stamp);

        match load_todos(path) {
            Ok(loaded) => {
                self.items = loaded.items;
                self.max_visible = loaded.max_visible;
                self.max_lines_per_item = loaded.max_lines_per_item;
                self.error = None;
            }
            Err(error) => {
                self.stamp = None;
                self.reset_content();
                self.error = Some(format!("TODO parse error: {error}"));
            }
        }
    }

    pub(super) fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub(super) fn is_visible(&self) -> bool {
        self.error.is_some() || !self.items.is_empty()
    }

    fn reset_content(&mut self) {
        self.items.clear();
        self.max_visible = DEFAULT_MAX_VISIBLE;
        self.max_lines_per_item = DEFAULT_MAX_LINES_PER_ITEM;
    }

    #[cfg(test)]
    pub(super) fn with_items(items: &[&str]) -> Self {
        Self {
            items: items.iter().map(|item| (*item).to_string()).collect(),
            ..Self::default()
        }
    }

    #[cfg(test)]
    fn for_path(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            ..Self::default()
        }
    }
}

fn global_todo_path(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|home| home.join(TODO_RELATIVE_PATH))
}

fn default_max_visible() -> usize {
    DEFAULT_MAX_VISIBLE
}

fn default_max_lines_per_item() -> usize {
    DEFAULT_MAX_LINES_PER_ITEM
}

fn load_todos(path: &Path) -> Result<LoadedTodos, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let document: TodoDocument =
        serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    let items = document
        .todos
        .into_iter()
        .filter(|todo| !todo.done)
        .map(|todo| todo.text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect();
    Ok(LoadedTodos {
        items,
        max_visible: document.max_visible.clamp(1, MAX_VISIBLE_LIMIT),
        max_lines_per_item: document
            .max_lines_per_item
            .clamp(1, MAX_LINES_PER_ITEM_LIMIT),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_only_non_empty_incomplete_todos() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(
            &path,
            r#"{
                "todos": [
                    {"text": "Review API", "done": false},
                    {"text": "Run tests"},
                    {"text": "Already done", "done": true},
                    {"text": "   ", "done": false}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            load_todos(&path).unwrap(),
            LoadedTodos {
                items: vec!["Review API".to_string(), "Run tests".to_string()],
                max_visible: 3,
                max_lines_per_item: 2,
            }
        );
    }

    #[test]
    fn loads_and_clamps_display_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(
            &path,
            r#"{
                "max_visible": 100,
                "max_lines_per_item": 0,
                "todos": [{"text": "Ship it"}]
            }"#,
        )
        .unwrap();

        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.max_visible, MAX_VISIBLE_LIMIT);
        assert_eq!(loaded.max_lines_per_item, 1);
    }

    #[test]
    fn resolves_todos_from_the_home_directory() {
        assert_eq!(
            global_todo_path(Some(Path::new("/home/user"))),
            Some(PathBuf::from("/home/user/.workmux/todos.json"))
        );
        assert_eq!(global_todo_path(None), None);
    }

    #[test]
    fn sync_clears_items_when_file_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, r#"{"todos":[{"text":"Ship it"}]}"#).unwrap();

        let mut panel = TodoPanel::for_path(path.clone());
        panel.sync(true);
        assert_eq!(panel.items, vec!["Ship it".to_string()]);
        assert_eq!(panel.max_visible, DEFAULT_MAX_VISIBLE);
        assert_eq!(panel.max_lines_per_item, DEFAULT_MAX_LINES_PER_ITEM);

        fs::remove_file(path).unwrap();
        panel.sync(false);
        assert!(panel.items.is_empty());
        assert!(!panel.is_visible());
    }
}
