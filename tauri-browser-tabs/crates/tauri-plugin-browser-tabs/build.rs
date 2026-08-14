const COMMANDS: &[&str] = &[
    "create_tab",
    "update_rect",
    "set_visible",
    "close_tab",
    "navigate",
    "list_tabs",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
