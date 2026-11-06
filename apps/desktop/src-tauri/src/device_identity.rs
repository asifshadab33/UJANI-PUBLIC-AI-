//! Local device identity backed by the operating-system credential vault.
//! Only the public key and its derived identifier are exposed to the frontend.
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ed25519_dalek::SigningKey;
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
