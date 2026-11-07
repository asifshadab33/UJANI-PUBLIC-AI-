//! Shared asynchronous native confirmation for security-sensitive desktop actions.
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

/// Show a native warning dialog and fail closed if its response channel closes.
pub(crate) async fn confirm_native(app: &AppHandle, title: &str, message: String) -> bool {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| { let _ = tx.send(approved); });
    rx.await.unwrap_or(false)
}
