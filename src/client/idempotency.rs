/// Idempotency başlık adı.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// UUIDv7 formatında benzersiz bir idempotency anahtarı üretir.
#[must_use]
pub fn generate_idempotency_key() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// İstemci seviyesinde idempotency anahtarını belirler.
///
/// Eğer kullanıcı veya ajan `--idempotency-key` ile bir anahtar vermişse o kullanılır.
/// Aksi halde yeni bir UUIDv7 anahtarı üretilir.
#[must_use]
pub fn resolve_idempotency_key(explicit: Option<&str>) -> String {
    explicit
        .filter(|k| !k.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(generate_idempotency_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_idempotency_key() {
        let key1 = generate_idempotency_key();
        let key2 = generate_idempotency_key();
        assert_ne!(key1, key2);
        assert!(uuid::Uuid::parse_str(&key1).is_ok());
    }

    #[test]
    fn test_resolve_idempotency_key() {
        let explicit = "my-custom-key-123";
        assert_eq!(resolve_idempotency_key(Some(explicit)), explicit);

        let generated = resolve_idempotency_key(None);
        assert!(uuid::Uuid::parse_str(&generated).is_ok());
    }
}
