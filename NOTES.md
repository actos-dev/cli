# Actos CLI — Geliştirme Notları ve Kararlar (NOTES.md)

Bu dosya, Actos CLI geliştirme sürecinde alınan mimari kararları, platform uyumluluğu notlarını ve kullanıcı/yönetici için önemli noktaları belgeler.

---

## 1. Mimari ve Bağımlılık Kararları

### 1.1 `actos-types` Bağımlılık Yolu (Local vs Git)
- **Karar**: Geliştirme aşamasında yerel çalışma alanının hızlı derlenmesi ve offline çalışabilirlik için `actos-types = { path = "../actos-backend/crates/actos-types", default-features = false }` olarak bağlandı.
- **Gerekçe**: Şartname (§0 ve Faz 0) gereği `openapi` / `utoipa` özellikleri kapalı tutulmalıdır. CI/CD veya yayın aşamasında `{ git = "https://github.com/actos-dev/backend" }` olarak revize edilebilir.

### 1.2 Güvenlik ve İzinler (0600)
- Config dosyası (`~/.config/actos/config.toml`) Unix sistemlerde `0600` (`-rw-------`) izinleriyle oluşturulur.
- Windows uyumluluğu için izin kontrolleri `#[cfg(unix)]` koruması altına alınmıştır.

---

## 2. Karar Günlüğü
- [2026-09-02] CLI geliştirme süreci Faz 0 ile başlatıldı.
