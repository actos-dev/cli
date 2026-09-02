# Actos CLI (`actos`)

Actos platformu için resmi komut satırı aracı (CLI).

`actos`, hem insanlar hem de yapay zekâ (AI) ajanları için tasarlanmış, öngörülebilir çıkış kodlarına (`PLAN.md` §4), katı `--json` sözleşmesine ve dayanıklı ağ yönetimine sahip bir CLI istemcisidir.

## Özellikler

- **Ajan Sözleşmesi**: `--json` bayrağı ile stdout'a yalnızca geçerli JSON basılır. İlerleme çubukları, uyarılar ve loglar stderr'e yönlendirilir.
- **Standart Çıkış Kodları**: Backend `ErrorCode` eşlemeleri ile tutarlı çıkış kodları (0: Başarı, 2: Kullanım hatası, 3: Kimlik doğrulama, 4: Yetki, 5: Bulunamadı, 6: Silinmiş içerik, 8: Doğrulama, 9: Hız sınırı, 10: Sunucu, 11: Ağ hatası).
- **Profiller ve Güvenlik**: `~/.config/actos/config.toml` dosyası `0600` izinleriyle korunur. API anahtarları konsol listelerinde otomatik maskelenir.
- **Platform Uyumluluğu**: Rust 2024 edition, MSRV 1.96, `#![forbid(unsafe_code)]`.

## Derleme ve Kurulum

Gereksinimler: Rust 1.96+

```bash
# Projeyi derleyin
cargo build --release

# Sürümü kontrol edin
cargo run -- --version
```

## Hızlı Başlangıç

### 1. Yapılandırma

```bash
# Profil yapılandırmasını görüntüleyin
actos config list

# API anahtarı veya adresi atayın
actos config set api_url https://api.actos.dev
actos config set api_key actos_your_api_key_here
```

Ortam değişkenleri profil değerlerini geçersiz kılabilir:
- `ACTOS_API_KEY`: API anahtarını geçersiz kılar
- `ACTOS_API_URL`: Sunucu adresini geçersiz kılar
- `ACTOS_PROFILE`: Kullanılacak profili seçer
- `ACTOS_CONFIG`: Özel konfigürasyon dosyası yolu

### 2. Kullanım

```bash
# CLI yardımını görüntüleyin
actos --help

# Makine-okunur formatta yardım (AI ajanları için)
actos --json --help
```

## Lisans

Bu proje **AGPL-3.0-only** lisansı altındadır. Detaylar için [LICENSE](LICENSE) dosyasına bakın.
