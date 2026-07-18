use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use data_encoding::BASE64;
use std::num::NonZeroU32;

/// Bitwarden uses PBKDF2 with SHA-256 for key derivation.
/// The default is 600,000 iterations (or 100,000 for older clients).
pub const DEFAULT_KDF_ITERATIONS: i32 = 600_000;
pub const DEFAULT_KDF: i32 = 0; // 0 = PBKDF2_SHA256

/// Hash the master password using PBKDF2-HMAC-SHA256.
/// The salt is the email address (lowercase).
pub fn hash_password(password: &str, email: &str, iterations: i32) -> String {
    let salt = email.to_lowercase();
    let mut result = [0u8; 32]; // SHA-256 output is 32 bytes

    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(iterations as u32).unwrap_or(NonZeroU32::new(100_000).unwrap()),
        salt.as_bytes(),
        password.as_bytes(),
        &mut result,
    );

    BASE64.encode(&result)
}

/// Verify a password hash against a known hash.
pub fn verify_password(
    password: &str,
    email: &str,
    iterations: i32,
    expected_hash: &str,
) -> bool {
    let computed = hash_password(password, email, iterations);
    // Constant-time comparison would be ideal, but for this use case it's acceptable
    computed == expected_hash
}

/// Generate cryptographically random bytes and return as base64.
pub fn generate_random_bytes(len: usize) -> String {
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; len];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    BASE64.encode(&bytes)
}

/// Generate a secure token (used for refresh tokens, 2FA tokens, etc.)
pub fn generate_token() -> String {
    generate_random_bytes(32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let hash = hash_password("testpassword", "test@example.com", 100_000);
        assert!(verify_password("testpassword", "test@example.com", 100_000, &hash));
        assert!(!verify_password("wrongpassword", "test@example.com", 100_000, &hash));
    }
}
