//! Local device identity backed by the operating-system credential vault.
//! Only the public key and its derived identifier are exposed to the frontend.
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::SigningKey;
use crate::native_consent::confirm_native;
use ujani_core::{peer_protocol::{SignedTaskEnvelope, TaskEnvelope, PROTOCOL_VERSION}, registered_operations};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{rngs::OsRng, RngCore};
use serde::Serialize;
use tauri::AppHandle;
use zeroize::Zeroizing;

const SERVICE: &str = "org.ujani.publicai";
const USER: &str = "device-identity-ed25519-v1";

/// Public Ed25519 identity fingerprint returned to the UJANI frontend.
/// Public identity material and fingerprint for this local installation.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub public_key_hex: String,
    pub algorithm: &'static str,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn signing_key_from_encoded(encoded: String) -> Result<SigningKey, String> {
    let encoded = Zeroizing::new(encoded);
    let decoded = Zeroizing::new(
        B64.decode(encoded.as_bytes())
            .map_err(|_| "Stored device identity is corrupted.".to_string())?,
    );
    let seed = Zeroizing::new(
        <[u8; 32]>::try_from(decoded.as_slice())
            .map_err(|_| "Stored device identity has an invalid key length.".to_string())?,
    );
    Ok(SigningKey::from_bytes(&seed))
}

pub(crate) fn load_existing_device_identity() -> Result<Option<DeviceIdentity>, String> {
    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
    match entry.get_password() {
        Ok(encoded) => {
            let signing = signing_key_from_encoded(encoded)?;
            let public = signing.verifying_key().to_bytes();
            Ok(Some(DeviceIdentity {
                device_id: ujani_core::sha256_hex(&public),
                public_key_hex: hex(&public),
                algorithm: "Ed25519",
            }))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Could not read device identity from OS credential vault: {e}")),
    }
}

fn load_or_create_signing_key(app: &AppHandle) -> Result<SigningKey, String> {
    if let Some(identity) = load_existing_device_identity()? {
        let public = ujani_core::sha256_hex(
            &hex_to_public_bytes(&identity.public_key_hex)?,
        );
        if public != identity.device_id {
            return Err("Stored device identity fingerprint is inconsistent.".into());
        }
        let entry = keyring::Entry::new(SERVICE, USER)
            .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
        return signing_key_from_encoded(
            entry.get_password().map_err(|e| format!("Could not read device identity from OS credential vault: {e}"))?,
        );
    }

    crate::with_local_state_receipt(app, "device_identity.create", b"create-if-absent", || {
        let entry = keyring::Entry::new(SERVICE, USER)
            .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
        let key = match entry.get_password() {
            Ok(encoded) => signing_key_from_encoded(encoded)?,
            Err(keyring::Error::NoEntry) => {
                let mut seed = Zeroizing::new([0_u8; 32]);
                let mut random = OsRng;
                random
                    .try_fill_bytes(&mut *seed)
                    .map_err(|_| "OS random source is unavailable.".to_string())?;
                let key = SigningKey::from_bytes(&seed);
                let encoded = Zeroizing::new(B64.encode(*seed));
                entry.set_password(encoded.as_str())
                    .map_err(|e| format!("Could not store device identity in OS credential vault: {e}"))?;
                key
            }
            Err(e) => return Err(format!("Could not read device identity from OS credential vault: {e}")),
        };
        let public = key.verifying_key().to_bytes();
        let output = serde_json::to_vec(&(ujani_core::sha256_hex(&public), hex(&public)))
            .map_err(|e| format!("Could not encode device identity receipt metadata: {e}"))?;
        Ok((key, output))
    })
}

fn hex_to_public_bytes(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("Stored device identity public key has an invalid length.".into());
    }
    let mut output = [0u8; 32];
    for (index, pair) in value.as_bytes().chunks(2).enumerate() {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        output[index] = (digit(pair[0]).ok_or("Stored device identity public key is invalid.")? << 4)
            | digit(pair[1]).ok_or("Stored device identity public key is invalid.")?;
    }
    Ok(output)
}

fn identity_from_signing_key(signing: &SigningKey) -> DeviceIdentity {
    let public = signing.verifying_key().to_bytes();
    DeviceIdentity {
        device_id: ujani_core::sha256_hex(&public),
        public_key_hex: hex(&public),
        algorithm: "Ed25519",
    }
}

/// Load or create the local Ed25519 signing key in the operating-system credential vault.
/// Returns or creates the local device identity without exporting its private key.
#[tauri::command]
pub fn get_or_create_device_identity(app: AppHandle) -> Result<DeviceIdentity, String> {
    let signing = load_or_create_signing_key(&app)?;
    Ok(identity_from_signing_key(&signing))
}



/// Rotate the local signing key after native confirmation; peers must re-enrol the new fingerprint.
/// Rotates the local device signing identity after native user confirmation.
#[tauri::command]
pub async fn rotate_device_identity(app: AppHandle) -> Result<DeviceIdentity, String> {
    let current = load_existing_device_identity()?
        .ok_or_else(|| "No device identity exists; create one before rotating it.".to_string())?;
    let approved = confirm_native(&app, "Confirm device identity rotation", format!("Rotate this device's Ed25519 identity?\n\nCurrent fingerprint: {}\n\nOther devices will need to re-enrol this device before peer tasks can be verified.", current.device_id), tokio_util::sync::CancellationToken::new(), std::time::Duration::from_secs(120)).await;
    if !approved {
        return Err("Device identity rotation was cancelled.".into());
    }
    let input = serde_json::to_vec(&current.device_id)
        .map_err(|e| format!("Could not encode device identity rotation metadata: {e}"))?;
    crate::with_local_state_receipt(&app, "device_identity.rotate", &input, || {
        let entry = keyring::Entry::new(SERVICE, USER)
            .map_err(|e| format!("OS credential vault unavailable: {e}"))?;
        let mut seed = Zeroizing::new([0_u8; 32]);
        let mut random = OsRng;
        random
            .try_fill_bytes(&mut *seed)
            .map_err(|_| "OS random source is unavailable.".to_string())?;
        let signing = SigningKey::from_bytes(&seed);
        let encoded = Zeroizing::new(B64.encode(*seed));
        entry.set_password(encoded.as_str())
            .map_err(|e| format!("Could not rotate device identity in OS credential vault: {e}"))?;
        let identity = identity_from_signing_key(&signing);
        let output = serde_json::to_vec(&(identity.device_id.as_str(), identity.public_key_hex.as_str()))
            .map_err(|e| format!("Could not encode rotated identity receipt metadata: {e}"))?;
        Ok((identity, output))
    })
}

/// Create a recipient-bound signed consent envelope after native approval; this does not transmit the payload.
/// Creates a signed consent envelope; it does not transmit the task payload.
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
    let approved = confirm_native(&app, "Approve peer task consent", format!("Approve preparing a signed UJANI peer task?\n\nOperation: {operation}\nRecipient: {recipient_device_id}\nTask: {task_id}\nPayload: {} bytes\nSHA-256: {digest}\nData scopes: {scopes}\n\nThis creates a signed consent envelope only; it does not send data.", payload.len()), tokio_util::sync::CancellationToken::new(), std::time::Duration::from_secs(120)).await;
    if !approved {
        return Err("Peer task signing was not approved. No data was sent.".into());
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_secs();
    // Two independent OS-CSPRNG words provide the 128-bit envelope nonce.
    let mut rng = OsRng;
    let mut nonce_bytes = [0u8; 16];
    rng.try_fill_bytes(&mut nonce_bytes)
        .map_err(|_| "OS random source is unavailable.".to_string())?;
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
    SignedTaskEnvelope::sign(envelope, &load_or_create_signing_key(&app)?)
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
        assert_eq!(hex_to_public_bytes(&encoded).unwrap(), public);
        assert!(hex_to_public_bytes("invalid").is_err());
    }
}
