// Generates this plugin's permission ACL from its command list, so an app
// embedding it can grant access via the standard "printers-sdk:default"
// capability instead of every command being silently denied (Tauri v2 denies
// all plugin commands by default unless a capability explicitly allows
// them -- unlike an app's OWN #[tauri::command] functions, which aren't
// ACL-gated). Without this build.rs, this plugin had no way to be invoked
// from a real app's webview at all.
const COMMANDS: &[&str] = &[
    "render_to_escpos",
    "render_to_html",
    "render_to_svg",
    "render_to_plain_text",
    "render_to_pdf",
    "validate_document",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
