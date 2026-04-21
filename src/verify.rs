use crate::Result;
use minisign_verify::{PublicKey, Signature};

/// Verifies a downloaded payload against a minisign public key and detached signature.
pub fn verify_minisign(payload: &[u8], pubkey: &str, signature: &str) -> Result<()> {
    let public_key = PublicKey::decode(pubkey)?;
    let signature = Signature::decode(signature)?;
    public_key.verify(payload, &signature, true)?;
    Ok(())
}
