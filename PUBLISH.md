# Yayın (publish) — CLI

> Durum: **yayınlanmadı, engelli.** Faz 0–17 kodlandı, kapılar yeşil
> (`cargo test`, `clippy -D warnings`, `fmt`). Kalan iş yalnızca paketleme
> (Faz 16).
>
> Son güncelleme: 2026-09-05.

## Hazır olanlar

- `CARGO_REGISTRY_TOKEN` GitHub secret'ı `actos-dev/cli` reposuna kondu.
- crates.io'da `actos`, `actos-cli`, `actos-types`, `actos-sdk` adlarının
  **hepsi müsait** (kontrol edildi 2026-09-05).

## ENGEL 1 — `actos-types` path bağımlılığı

`Cargo.toml:18`:

```toml
actos-types = { path = "../actos-backend/crates/actos-types", default-features = false }
```

`cargo publish` path bağımlılığı olan bir crate'i **kabul etmez** — yani
`cargo install actos` bugün çalışmaz. Aynı bağımlılığı Rust SDK'sı da
taşıyor; ikisi birlikte çözülecek.

Çözüm: `actos-types` crates.io'ya yayınlandıktan sonra buradaki satır
sürüm bağımlılığına çevrilir:

```toml
actos-types = { version = "0.1", default-features = false }
```

## ENGEL 2 — crates.io ad çakışması

`cli/Cargo.toml` ve `rust/Cargo.toml` **ikisi de** `name = "actos"` diyor.
crates.io'da bu adın tek sahibi olabilir.

Alışıldık çözüm: kütüphane `actos` adını alır, CLI `actos-cli` olarak
yayınlanır — ama `[[bin]] name = "actos"` sayesinde kullanıcı yine `actos`
komutunu alır:

```toml
[package]
name = "actos-cli"

[[bin]]
name = "actos"
path = "src/main.rs"
```

Böylece `cargo install actos-cli` kurar, komut `actos` olur. Karar
verilmedi.

## Küçük iş — açıklama hâlâ Türkçe

`Cargo.toml`:

```toml
description = "Actos platformu için resmi komut satırı aracı"
```

Bu metin crates.io paket sayfasında görünüyor, yani dışa dönük. Projenin
ana dili İngilizce (backend Faz 20'de karara bağlandı) — çevrilmeli.

## Lisans notu

Bu CLI `AGPL-3.0-only`. Bir uygulama (kütüphane değil) olduğu için AGPL
burada sorun değil — bulaşma sorunu yalnızca kütüphanelerde geçerli.
`actos-types` Apache-2.0'a çevrildiğinde (kullanıcı kararı, bkz. Rust
SDK'sının `PUBLISH.md`'si) AGPL bir uygulamanın Apache bir kütüphaneyi
kullanması sorunsuz.

## Yayın iş akışı henüz yazılmadı

`.github/workflows/` dizini **hiç yok**. Diğer repolarda en azından
`ci.yml` var; burada o da yazılacak.

## Sıradaki adım

1. `actos-types` Apache-2.0'a çevrilsin ve crates.io'ya yayınlansın
   (backend repo'sunun işi).
2. Ad çakışmasını çöz — `actos` kütüphaneye mi CLI'a mı gidiyor.
3. Path bağımlılığını sürüm bağımlılığına çevir.
4. `description`'ı İngilizceye çevir.
5. `ci.yml` + publish workflow'u + `v0.1.0` tag'i.

---

## Sonraki tur için fikir — CLI, Rust SDK'sını kullanmalı

**Kullanıcı gözlemi (2026-09-05): "rust sdk'sını neden kullanmıyor ki?"**
Haklı bir soru; ölçüldü ve gerçek bir tekrar var.

CLI şu an `actos-types` + `reqwest` alıp **kendi taşıma katmanını**
yazıyor: `src/client/` altında 799 satır — `retry.rs`, `ratelimit.rs`,
`idempotency.rs`, `pagination.rs`, `mod.rs`. Rust SDK'sı (7 424 satır) tam
olarak aynı işleri yapıyor ve üstüne 14 kaynak modülü (`actors`, `auth`,
`comments`, `feed`, `inbox`, `admin`, `meta`, ...) sunuyor.

Yani retry politikası, hız sınırı başlıklarının okunması, idempotency
anahtarı üretimi ve cursor sayfalaması **iki yerde ayrı ayrı yazılmış
durumda.** Sözleşme değiştiğinde ikisinin de güncellenmesi gerekiyor ve
biri unutulursa sapma sessiz olur — bu projede tam olarak bu tür bir
sapma daha önce yaşandı (`[silindi]` / `[deleted]`).

Doğrusu: CLI `actos` crate'ine (Rust SDK) bağımlı olsun, `src/client/`
silinsin. CLI'a kalan iş argüman ayrıştırma, çıktı biçimlendirme ve TUI —
zaten olması gereken sorumluluk.

**Neden şimdi değil:** bu bir refactor, yayın işi değil. Önce paketleme
bitsin (yukarıdaki maddeler), `actos` crate'i crates.io'da yayında olsun,
sonra CLI ona geçsin. Sırayı tersine çevirmek yayını geciktirir.

Not edilme sebebi: unutulmasın. Acelesi yok.
