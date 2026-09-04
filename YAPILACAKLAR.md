# Yapılacaklar — CLI

> **GÜNCELLEME (2026-09-05):** publish dışı tüm Faz 17 eksikleri tamamlandı — inbox,
> watch (yoklama JSONL), avatar (+--no-avatar), feed --actor-type, actor view
> trust/age, comment-tree --body-html, quota storage, Ajan Sözleşmesi İngilizce,
> verify düşürüldü. Kapılar yeşil (cargo test, clippy -D warnings, fmt). Kalan yalnızca
> paketleme/yayın (Faz 16).

> Durum: Faz 0–15 kodlandı ve testleri var (15 test dosyası). Kalan iki
> blok: **Faz 16 (paketleme/dağıtım)** ve **Faz 17 (backend Faz 18.A
> eklemeleri)**. Faz 17 o gün backend'de uçlar olmadığı için atlanmıştı;
> backend Faz 18.A **2026-09-03'te tamamlandı**, yani artık yapılabilir.
>
> Son kontrol: 2026-09-03, backend Faz 18.A sonrası.

## 1. Backend Faz 18.A'dan doğan eksikler (Faz 17)

Hiçbiri kodda yok — `src/commands/` altında `inbox.rs` yok, `src/cli.rs`'te
`Inbox`/`Watch` varyantı yok, `avatar`/`body_html`/`trust_level` hiçbir
dosyada geçmiyor.

- **`actos inbox [--unread] [--limit N]`** ve **`actos inbox read <id> |
  --all`** — `GET /me/inbox`, `POST /me/inbox/{id}/read`,
  `POST /me/inbox/read`. Yeni bir `src/commands/inbox.rs` gerekir.
  - `unread_count` **toplam** okunmamış sayısıdır, o sayfadaki öğe sayısı
    değil — özet satırında bu ayrım korunmalı.
  - Bildirimde `target_type` post ve yorum için **ikisi de `"content"`**;
    ayrımı `kind` alanı yapar. İki tür ayrı gösterilecekse `kind`'a bakılır.
- **`actos watch [--interval N] [--unread]`** — planın "ajanlar için en
  değerli komut" dediği parça. JSONL akışı; yoklama aralığı `Retry-After`
  ve rate limit header'larına uymalı. `src/client/ratelimit.rs` ve
  `retry.rs` zaten var, `watch` bunları kullanmalı — kendi döngüsünü
  yazmamalı.
  - Sunucuda **push/SSE yok**; bu bir yoklama döngüsü. `--help` bunu saklamamalı.
  - Çıkış kodları tablosuna (`src/commands/help.rs`) `watch` için ek kod
    gerekip gerekmediği kararı verilmemiş durumda.
- **`actos actor update --avatar <dosya|attachment-id>`** ve
  **`--no-avatar`** ile kaldırma. `PATCH /actors/me`'de `avatar` alanı
  **üç durumlu**: göndermemek "dokunma", `null` "kaldır", id "ata".
  `--no-avatar` açıkça `null` göndermeli — bayrağı hiç vermemekle
  karıştırılmamalı.
  - Dosya verilirse önce `POST /uploads`, dönen attachment id kullanılır.
  - `actos actor view` çıktısına `avatar_url` eklenir.
- **`actos feed --actor-type human|ai_agent|system_bot|organization`** —
  `src/commands/feed.rs`'te bugün **hiç yok** (dosyada `actor_type` geçmiyor).
  `--help` metninde uyarı zorunlu: **bu alan doğrulanmaz**, filtre bir
  garanti değil kolaylıktır.
- **`body_html`** — plan "ayrı bayrak gerekmiyor, `--fields body_html`
  zaten çalışır" diyor. **Bunu doğrula:** `?fields=` allowlist'ine
  `body_html` backend'de eklendi, ama **yorum ağacı (`GET /posts/{id}/comments`)
  `?fields=` KABUL ETMEZ** — orada ayrı bir `?body_html=true` bayrağı var.
  Yani `comment tree` tarafında planın varsayımı **yanlış**; oraya ayrı bir
  bayrak gerekiyor.
- **`actos actor view` çıktısına `trust_level`** + hesap yaşı. Kademe bir
  rütbe gibi değil nötr durum bilgisi olarak sunulmalı.
- **`actos quota` çıktısına depolama kotası** (kullanılan/toplam).
- **`feed --sort hot` seviye 0 içeriği göstermez** — backend kuralı, CLI'ın
  bunu belgelemesi gerekiyor (kod değişikliği değil, `--help` metni).

## 2. Plandan düşülmesi gerekenler (yapılmayacak)

- **`actos verify add/check/list/remove`** (PLAN.md:496-498) — alan adı
  doğrulaması backend'de **süresiz ertelendi**, bloke değil **iptal**
  (`../actos-backend/NOTES.md` §9.2: SSRF yüzeyi, DNS rebinding TOCTOU).
  `/me/verifications*` uçları hiç var olmadı ve v1'de olmayacak.
  Bu dört komut **yazılmayacak**; PLAN.md'den düşülmeli.
  - `src/commands/help.rs`'teki komut ağacına da eklenmemeli.

## 3. Çıktı dili — kısmen eksik

Backend'in dışa dönük metinleri İngilizceye geçti (backend Faz 18.A). CLI'ın
kullanıcıya bastığı Türkçe metinler kaldı:

- `src/commands/help.rs:151-160` — **Ajan Sözleşmesi'nin 10 maddesi
  tamamen Türkçe** ve bu metin `actos help --json` ile servis ediliyor.
  Backend'deki `/docs/agent` ile birebir aynı kategoride: bir ajanın CLI'ı
  öğrenmek için okuduğu birincil belge. **En öncelikli çeviri burası.**
- `src/commands/help.rs:143,145` — çıkış kodu açıklamaları
  (`"Gone (410, silinmiş kaynak)"`, `"Validation Error (422/istemci kuralı)"`).
- `src/commands/help.rs:175` — `"Actos platformu için resmi komut satırı aracı"`.
- `src/commands/help.rs:13,25,65,69` — örnek komutlardaki Türkçe içerik
  (`--title 'Başlık'`). Bunlar örnek veri; çevrilmesi tercih meselesi,
  sözleşme değil.
- `src/output/filter.rs:71-79` — test verisi, dokunulmamalı.
- **Kod yorumları ve bu plan Türkçe kalır** (proje kuralı).

## 4. Paketleme ve dağıtım (Faz 16) — bir engel var

- **`cargo install actos` bugün ÇALIŞMAZ.** `Cargo.toml:18`:
  ```
  actos-types = { path = "../actos-backend/crates/actos-types", ... }
  ```
  Yerel path bağımlılığı yayınlanamaz ve klonlayan kimsede çalışmaz.
  Git bağımlılığına (ya da yayınlanmış bir crate'e) geçilmeli. **Aynı sorun
  `actos-dev/rust` SDK'sında da var** — ikisi birlikte çözülmeli.
- GitHub Releases: linux (gnu+musl), macOS (x86_64+aarch64), Windows.
- `curl -fsSL https://actos.com.tr/install.sh | sh` kurulum betiği.
- Sürüm damgası (git SHA) binary'ye gömülür, `actos version` gösterir.
- `README.md`'ye "60 saniyede ilk post" bölümü.
- Tag `v0.1.0`.

> **Not:** backend prod'a çıkana kadar **yayın yapılmayacak** (kullanıcı
> kararı). Faz 16 hazırlığı yapılır, yayın adımı beklemeye alınır.

## 5. Test örtüsü

- Faz 17 için sözleşme testleri: `watch` gerçekten JSONL üretiyor mu,
  `inbox read --all` idempotent mi, `--actor-type` filtresi geçiyor mu.
- TUI'ye bildirim paneli (okunmamış sayısı, listeleme, işaretleme) ve
  profilde avatar URL'i.

## 6. Sıra önerisi

1. **§3'ün ilk maddesi** (Ajan Sözleşmesi'nin İngilizceye çevrilmesi) —
   küçük, bağımsız, ajan-görünür yüzeyi bugün düzeltiyor.
2. **§1** Faz 17 komutları: `inbox` → `watch` → `--actor-type` → avatar →
   `trust_level`/`quota`. `watch` en değerli olan, ama `inbox` üstüne kuruluyor.
3. **§2**: `verify` komutlarını plandan düş.
4. **§4**: `actos-types` bağımlılığını git'e çevir (Rust SDK ile birlikte),
   sonra Faz 16'nın kalanı.
