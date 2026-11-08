//! Shared asynchronous native confirmation for security-sensitive desktop actions.
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio_util::sync::CancellationToken;

/// Show a native warning dialog and fail closed on cancellation, timeout or channel closure.
/// The dialog plugin does not expose a dismiss handle; cancellation still guarantees that
/// the pending confirmation can never authorize a network request.
pub(crate) async fn confirm_native(
    app: &AppHandle,
    title: &str,
    message: String,
    cancellation: CancellationToken,
    timeout: Duration,
) -> bool {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| { let _ = tx.send(approved); });

    tokio::select! {
        _ = cancellation.cancelled() => false,
        result = tokio::time::timeout(timeout, rx) => {
            matches!(result, Ok(Ok(true)))
        }
    }
}
