use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|error| anyhow::anyhow!("hashing password: {error}"))?;
    Ok(hash.to_string())
}

fn is_bcrypt(stored_hash: &str) -> bool {
    ["$2y$", "$2a$", "$2b$"]
        .iter()
        .any(|prefix| stored_hash.starts_with(prefix))
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    if is_bcrypt(stored_hash) {
        return bcrypt::verify(password, stored_hash).unwrap_or(false);
    }
    PasswordHash::new(stored_hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn needs_rehash(stored_hash: &str) -> bool {
    is_bcrypt(stored_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_hashes_verify() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!needs_rehash(&hash));
    }

    #[test]
    fn imported_bcrypt_hashes_verify_and_need_rehash() {
        let php_style = bcrypt::hash("geheim-wachtwoord", 4)
            .unwrap()
            .replacen("$2b$", "$2y$", 1);
        assert!(verify_password("geheim-wachtwoord", &php_style));
        assert!(!verify_password("fout", &php_style));
        assert!(needs_rehash(&php_style));
    }
}
