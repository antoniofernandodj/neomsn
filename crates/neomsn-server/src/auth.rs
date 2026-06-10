use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const JWT_SECRET: &[u8] = b"neomsn_secret_change_in_production";
const TOKEN_EXPIRY_SECS: usize = 30 * 24 * 3600; // 30 days

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // user_id
    pub device_id: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn issue_token(user_id: Uuid, device_id: Uuid) -> Result<String> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        device_id: device_id.to_string(),
        iat: now,
        exp: now + TOKEN_EXPIRY_SECS,
    };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))?;
    Ok(token)
}

pub fn verify_token(token: &str) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}
