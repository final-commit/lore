use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Shortcut { pub category: &'static str, pub action: &'static str, pub keys: &'static str }

pub async fn list_shortcuts() -> Json<Vec<Shortcut>> {
    Json(vec![
        Shortcut { category: "Navigation", action: "Search", keys: "Cmd+K" },
        Shortcut { category: "Navigation", action: "New document", keys: "Cmd+N" },
        Shortcut { category: "Navigation", action: "Toggle sidebar", keys: "Cmd+\\" },
        Shortcut { category: "Editor", action: "Bold", keys: "Cmd+B" },
        Shortcut { category: "Editor", action: "Italic", keys: "Cmd+I" },
        Shortcut { category: "Editor", action: "Heading 1", keys: "Cmd+Alt+1" },
        Shortcut { category: "Editor", action: "Heading 2", keys: "Cmd+Alt+2" },
        Shortcut { category: "Editor", action: "Heading 3", keys: "Cmd+Alt+3" },
        Shortcut { category: "Editor", action: "Code block", keys: "Cmd+Alt+C" },
        Shortcut { category: "Editor", action: "Link", keys: "Cmd+K" },
        Shortcut { category: "Editor", action: "Bullet list", keys: "Cmd+Shift+8" },
        Shortcut { category: "Editor", action: "Ordered list", keys: "Cmd+Shift+9" },
        Shortcut { category: "Editor", action: "Slash commands", keys: "/" },
        Shortcut { category: "General", action: "Save", keys: "Cmd+S" },
        Shortcut { category: "General", action: "Close/Escape", keys: "Esc" },
        Shortcut { category: "General", action: "Help", keys: "Cmd+?" },
        Shortcut { category: "General", action: "Undo", keys: "Cmd+Z" },
        Shortcut { category: "General", action: "Redo", keys: "Cmd+Shift+Z" },
    ])
}
