//! Local device identity backed by the operating-system credential vault.
//! Only the public key and its derived identifier are exposed to the frontend.
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::SigningKey;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use ujani_core::{peer_protocol::{SignedTaskEnvelope, TaskEnvelope, PROTOCOL_VERSION}, registered_operations};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use tauri::AppHandle;
use zeroize::Zeroizing;

const SERVICE: &str = "org.ujani.publicai";
const USER: &str = "device-identity-ed25519-v1";

#[derive(Debug, Clone, Serialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub public_key_hex: String,
    pub algorithm: &'static str,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn load_or_create_signing_key() -> Result<SigningKey, String> {
    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
    match entry.get_password() {
        Ok(encoded) => {
            let encoded = Zeroizing::new(encoded);
            let decoded = Zeroizing::new(B64.decode(encoded.as_bytes())
                .map_err(|_| "Stored device identity is corrupted.".to_string())?);
            let seed: [u8; 32] = decoded.as_slice().try_into()
                .map_err(|_| "Stored device identity has an invalid key length.".to_string())?;
            Ok(SigningKey::from_bytes(&seed))
        }
        Err(keyring::Error::NoEntry) => {
            let mut seed = Zeroizing::new([0_u8; 32]);
            OsRng.fill_bytes(&mut *seed);
            let key = SigningKey::from_bytes(&seed);
            let encoded = Zeroizing::new(B64.encode(&*seed));
            entry.set_password(encoded.as_str())
                .map_err(|e| format!("Could not store device identity in OS credential vault: {e}"))?;
            Ok(key)
        }
        Err(e) => Err(format!("Could not read device identity from OS credential vault: {e}")),
    }
}

#[tauri::command]
pub fn get_or_create_device_identity(_app: AppHandle) -> Result<DeviceIdentity, String> {
    let signing = load_or_create_signing_key()?;
    let public = signing.verifying_key().to_bytes();
    let public_key_hex = hex(&public);
    let device_id = ujani_core::sha256_hex(&public);
    Ok(DeviceIdentity { device_id, public_key_hex, algorithm: "Ed25519" })
}



#[tauri::command]
pub async fn rotate_device_identity(app: AppHandle) -> Result<DeviceIdentity, String> {
    let current = get_or_create_device_identity(app.clone())?;
    let approved = app.dialog()
        .message(format!("Rotate this device's Ed25519 identity?\n\nCurrent fingerprint: {}\n\nOther devices will need to re-enrol this device before peer tasks can be verified.", current.device_id))
        .title("Confirm device identity rotation")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    if !approved {
        return Err("Device identity rotation was cancelled.".into());
    }
    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
    let mut seed = Zeroizing::new([0_u8; 32]);
    OsRng.fill_bytes(&mut *seed);
    let signing = SigningKey::from_bytes(&seed);
    let encoded = Zeroizing::new(B64.encode(&*seed));
    entry.set_password(encoded.as_str())
        .map_err(|e| format!("Could not rotate device identity in OS credential vault: {e}"))?;
    let public = signing.verifying_key().to_bytes();
    let public_key_hex = hex(&public);
    let device_id = ujani_core::sha256_hex(&public);
    Ok(DeviceIdentity { device_id, public_key_hex, algorithm: "Ed25519" })
}

#[tauri::command]
pub async fn sign_peer_task_envelope(
    app: AppHandle,
    task_id: String,
    operation: String,
    recipient_device_id: String,
    payload: Vec<u8>,
    requested_scopes: Vec<String>,
) -> Result<SignedTaskEnvelope, String> {
    if requested_scopes.is_empty() || requested_scopes.len() > 3
        || requested_scopes.iter().any(|scope| {
            !matches!(scope.as_str(), "prompt.read" | "document.read" | "result.write")
        }) {
        return Err("Peer task must request one to three registered data scopes only.".into());
    }
    if payload.len() > 10 * 1024 * 1024 {
        return Err("Peer payload exceeds the 10 MiB signing limit.".into());
    }
    if recipient_device_id.len() != 64 || !recipient_device_id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("Recipient must be a 64-character lowercase device fingerprint.".into());
    }
    if !crate::security_store::is_active_trusted_peer(&app, &recipient_device_id)? {
        return Err("Recipient is not enrolled as an active trusted peer on this device.".into());
    }
    if !registered_operations().iter().any(|registered| registered.id == operation) {
        return Err("Peer operation is not in UJANI's registered operation catalogue.".into());
    }
    if task_id.is_empty() || task_id.len() > 96
        || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
        return Err("Invalid peer task identifier.".into());
    }
    let scopes = requested_scopes.join(", ");
    let digest = ujani_core::sha256_hex(&payload);
    let approved = app.dialog()
        .message(format!("Approve preparing a signed UJANI peer task?\n\nOperation: {operation}\nRecipient: {recipient_device_id}\nTask: {task_id}\nPayload: {} bytes\nSHA-256: {digest}\nData scopes: {scopes}\n\nThis creates a signed consent envelope only; it does not send data.", payload.len()))
        .title("Approve peer task consent")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    if !approved {
        return Err("Peer task signing was not approved. No data was sent.".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_secs();
    let mut nonce_bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = hex(&nonce_bytes);
    let envelope = TaskEnvelope {
        protocol_version: PROTOCOL_VERSION,
        task_id,
        recipient_device_id,
        issued_at: now,
        expires_at: now.saturating_add(ujani_core::peer_protocol::MAX_TASK_TTL_SECONDS),
        nonce,
        operation,
        input_sha256: digest,
        requested_scopes,
        user_consent: true,
    };
    SignedTaskEnvelope::sign(envelope, &load_or_create_signing_key()?)
        .map_err(|e| format!("Could not sign peer task envelope: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_hex_is_fixed_width_and_lowercase() {
        let key = SigningKey::from_bytes(&[7_u8; 32]);
        let public = key.verifying_key().to_bytes();
        let encoded = hex(&public);
        assert_eq!(encoded.len(), 64);
        assert!(encoded.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        assert_eq!(ujani_core::sha256_hex(&public).len(), 64);
    }
}
