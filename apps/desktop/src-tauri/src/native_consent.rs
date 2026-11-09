//! Shared asynchronous native confirmation for security-sensitive desktop actions.
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// Resolve a native approval response, failing closed for every non-approval outcome.
async fn await_native_approval(
    approval: oneshot::Receiver<bool>,
    cancellation: CancellationToken,
    timeout: Duration,
) -> bool {
    tokio::select! {
        _ = cancellation.cancelled() => false,
        result = tokio::time::timeout(timeout, approval) => {
            matches!(result, Ok(Ok(true)))
        }
    }
}

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
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .show(move |approved| { let _ = tx.send(approved); });

    await_native_approval(rx, cancellation, timeout).await
}

#[cfg(test)]
mod tests {
    use super::await_native_approval;
    use std::time::Duration;
    use tokio::sync::oneshot;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn explicit_approval_is_the_only_successful_outcome() {
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(await_native_approval(
            rx, CancellationToken::new(), Duration::from_secs(1),
        ));
        tx.send(true).unwrap();
        assert!(task.await.unwrap());
    }

    #[tokio::test]
    async fn denial_channel_closure_timeout_and_cancellation_fail_closed() {
        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(await_native_approval(
            rx, CancellationToken::new(), Duration::from_secs(1),
        ));
        tx.send(false).unwrap();
        assert!(!task.await.unwrap());

        let (tx, rx) = oneshot::channel::<bool>();
        drop(tx);
        assert!(!await_native_approval(
            rx, CancellationToken::new(), Duration::from_secs(1),
        ).await);

        let (_tx, rx) = oneshot::channel::<bool>();
        assert!(!await_native_approval(
            rx, CancellationToken::new(), Duration::from_millis(1),
        ).await);

        let (tx, rx) = oneshot::channel::<bool>();
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(!await_native_approval(rx, cancellation, Duration::from_secs(1)).await);
        drop(tx);
    }
}
