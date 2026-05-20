//! 密码哈希

use argon2::{
    password_hash::{
        PasswordHash, PasswordHasher as ArgonPasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2,
};
use rand::rngs::OsRng;

/// 密码哈希器
pub struct PasswordHasher;

impl PasswordHasher {
    /// 哈希密码
    pub fn hash(password: &str) -> Result<String, argon2::password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(hash.to_string())
    }

    /// 验证密码
    pub fn verify(password: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash() {
        let password = "secure_password_123";
        let hash = PasswordHasher::hash(password).unwrap();

        // 哈希应该是非空的
        assert!(!hash.is_empty());
        // 哈希应该不同于原始密码
        assert_ne!(hash, password);
    }

    #[test]
    fn test_password_verify() {
        let password = "secure_password_123";
        let hash = PasswordHasher::hash(password).unwrap();

        // 正确密码应该验证通过
        assert!(PasswordHasher::verify(password, &hash));
        // 错误密码应该验证失败
        assert!(!PasswordHasher::verify("wrong_password", &hash));
    }

    #[test]
    fn test_password_not_hashed_same() {
        // 两次哈希同一个密码应该得到不同的哈希值（因为 salt 不同）
        let password = "secure_password_123";
        let hash1 = PasswordHasher::hash(password).unwrap();
        let hash2 = PasswordHasher::hash(password).unwrap();

        assert_ne!(hash1, hash2);
        // 但两者都应该能验证通过
        assert!(PasswordHasher::verify(password, &hash1));
        assert!(PasswordHasher::verify(password, &hash2));
    }
}
