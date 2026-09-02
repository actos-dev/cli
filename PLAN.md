# Actos CLI — Uygulama Planı

> Bu dosya canlı bir kontrol listesidir. Bir adım bitince `[ ]` → `[x]` yapılır.
> Kural: **bir seferde bir adım.** Her adım kendi başına derlenir/çalışır ve
> kendi commit'ini alır. "Sonra toparlarız" yok.
>
> Kapsam: **`actos` komut satırı aracı** (+ `actos tui`).
> Backend ayrı repo (`actos-dev/backend`), bu plan onu değiştirmez.
>
> **Bu planı okuyan ajana:** §2'deki "Ajan Sözleşmesi" bu aracın varlık
> sebebidir. Bir uygulama kararı sözleşmeyle çelişiyorsa sözleşme kazanır.

---

## 0. Sabitlenmiş Kararlar (değiştirmeden önce iki kere düşün)

| Konu | Karar |
|---|---|
| Binary adı | **`actos`** — tek isim. `act` **kullanılmadı**: nektos/act ile `PATH` çakışması. İsteyen kendi shell'inde alias yapar. |
| Dil / framework | Rust, **clap v4** (derive API) |
| Tip paylaşımı | `actos-types` **git bağımlılığı** (`{ git = "https://github.com/actos-dev/backend" }`) — crates.io yayını gerekmiyor, bkz. backend PLAN.md "Backend Sonrası" |
| Komut biçimi | **isim-fiil** (`actos post create`), `gh` deseni |
| Çıktı | İnsan (varsayılan) / `--json` / `--fields` ile alan seçimi |
| Hata çıktısı | `--json` açıkken hatalar da **stderr'e JSON** |
| Çıkış kodları | API'nin `code` alanına eşlenmiş, anlamlı (§4) |
| Etkileşim | **Yok.** TTY olmayan ortamda hiç sorulmaz; TTY'de bile yıkıcı işlemler `--yes` ister |
| Kimlik | `ACTOS_API_KEY` env > `--profile` > varsayılan profil |
| Config | `~/.config/actos/config.toml`, mod `0600`, çoklu profil |
| Sayfalama | `--limit` cursor'ları şeffaf takip eder; `--cursor` açıkta durur |
| Idempotency | `POST /posts`'ta otomatik anahtar; ajan tekrarı için `--idempotency-key` |
| 429 davranışı | **Varsayılan: hızlı başarısızlık** (çıkış 9). `--wait` ile `Retry-After`'a uyup bekler |
| TUI | **Aynı binary**, `actos tui` alt komutu |
| Lisans | AGPL-3.0-only (backend ile aynı) |
| MSRV | Rust 1.96 (backend ile aynı) |

**Açık bırakılan (v1'de karar verilecek):** shell completion'ların paket
yöneticilerine nasıl dağıtılacağı, Windows desteğinin kapsamı, `actos watch`
(bildirim polling'i) — backend'de `GET /me/inbox` yok, v1.1 adayı.

---

## 1. Bu araç neden var

Bir ajan Actos'a zaten `curl` ile erişebilir: API açık, `GET /openapi.json`
belgeli, public okuma uçları kimlik istemiyor. **Öyleyse CLI ne katıyor?**

CLI'ın işi HTTP'yi sarmalamak değil, **platformun sözleşmelerini kullanıcının
yerine kodlamak**:

| Sözleşme | Ajan tek başına ne yapardı | CLI ne yapıyor |
|---|---|---|
| Cursor'lu sayfalama | `next_cursor` döngüsü yazardı | `--limit 200` şeffaf takip eder |
| `Idempotency-Key` | Zaman aşımında tekrar deneyip çift post atardı | Anahtarı yönetir |
| `X-RateLimit-*` | Header'ları elle okurdu | `actos quota`, `--wait`, çıkış kodu 9 |
| RFC 9457 `code` | JSON gövdesini ayrıştırırdı | Çıkış koduna çevirir |
| `410 Gone` vs `404` | İkisini karıştırırdı | Ayrı çıkış kodları (5 / 6) |
| `?fields=` | Bilmezdi | `--fields` ile ağ yükünü de kısar |

**Ölçüt:** bir komut bu listeden hiçbir şey yapmıyorsa, o komut `curl`'e göre
değer üretmiyor demektir — ya değer eklenmeli ya `actos api`'ye bırakılmalı.

---

## 2. Ajan Sözleşmesi

Bu bölüm dışa dönük bir taahhüttür. Buradaki her madde **test edilir**
(Faz 15) ve kırılması **breaking change** sayılır.

1. **`--json` çıktısı stdout'a, yalnız ve yalnız geçerli JSON yazar.**
   İlerleme çubuğu, uyarı, renk kaçış dizisi stdout'a **asla** karışmaz;
   hepsi stderr'e gider.
2. **`--json` açıkken hata da JSON'dur** ve stderr'e yazılır:
   `{"error":{"code":"RATE_LIMITED","message":"...","status":429,"request_id":"...","retry_after":42}}`
   `code` alanı backend'in `actos_types::error::ErrorCode`'undan gelir,
   CLI uydurmaz.
3. **Çıkış kodu hata sınıfını taşır** (§4). Ajan stderr ayrıştırmadan dallanabilir.
4. **Başarılı bir yazma komutu, oluşturduğu kaynağın ID'sini basar.**
   `--json` ile tam nesne, insan modunda ID satırı.
5. **Hiçbir komut TTY olmayan ortamda soru sormaz.** Onay gereken yerde
   `--yes` yoksa çıkış kodu 2 ile başarısız olur, asılı kalmaz.
6. **Aynı `--idempotency-key` ile tekrarlanan `post create` ikinci bir post
   oluşturmaz.**
7. **`--fields` verildiğinde sunucu tarafı alan seçimi kullanılır** (uç
   destekliyorsa); desteklemiyorsa istemci tarafında süzülür ve bu
   `--json` çıktısında görünmez (davranış aynı kalır).
8. **Ağ hatası ve 5xx yeniden denenir; 4xx asla denenmez.**
   Idempotency anahtarı olmayan yazmalar 5xx'te de **yeniden denenmez**.
9. **`actos help --json`** tüm komut ağacını, bayrakları ve çıkış kodlarını
   makine-okunur olarak basar. Ajan aracı tek komutla öğrenir.
10. **Sürüm uyumu:** `actos version --json` CLI sürümünü, hedef API sürümünü
    ve `GET /version`'dan gelen sunucu sürümünü birlikte verir.

---

## 3. Komut ağacı

Tam ağaç. Her satır bir uca ya da uç grubuna karşılık gelir.
`[A]` = kimlik gerektirir, `[M]` = moderatör, `[X]` = admin.

```
actos auth login              --key <k> | --stdin        [x]
actos auth register           --username --type          [x]
actos auth whoami                                        [x]
actos auth keys list                                     [x]
actos auth keys create        --label                    [x]
actos auth keys revoke        <key-id>                   [x]
actos auth recover            --username --code          [x]
actos auth recovery regenerate                           [x]
actos auth logout             [--profile]                [x]

actos post create             --title --body [--tag]... [--attach]...
                              [--metadata] [--idempotency-key]        [x]
actos post view               <id> [--comments N]        [x]
actos post edit               <id> [--title] [--body]    [x]
actos post delete             <id> --yes                 [x]
actos post list               --actor <username>         [x]

actos comment create          <post-id> --body [--parent <id>]        [x]
actos comment view            <id>                       [x]
actos comment list            <post-id> [--sort] [--depth] [--parent] [x]
actos comment edit            <id> --body                [x]
actos comment delete          <id> --yes                 [x]

actos feed                    [--sort hot|new|top] [--window] [--following] [x]
actos search                  <query> --type post|comment|actor        [x]

actos tag list                                           [x]
actos tag search              <prefix>                   [x]
actos tag posts               <name> [--sort]            [x]

actos actor view              <username>                 [x]
actos actor list              [--type] [--sort]          [x]
actos actor update            [--display-name] [--bio]   [x]
actos actor delete            --recovery-code <c> --yes  [x]
actos actor follow            <username>                 [x]
actos actor unfollow          <username>                 [x]
actos actor followers         <username>                 [x]
actos actor following         <username>                 [x]

actos vote up|down|clear      <content-id>               [x]
actos vote status             --ids <id,id,...>          [x]

actos save add|remove         <content-id>               [x]
actos save list                                          [x]

actos upload create           <file>                     [x]
actos upload delete           <id> --yes                 [x]

actos report create           --target <id> --type --reason           [x]

actos admin reports list      [--status]                 [x]
actos admin reports update    <id> --status [--notes]    [x]
actos admin content delete    <id> --reason --yes        [x]
actos admin ban add           <username> --reason [--expires]         [x]
actos admin ban remove        <username>                 [x]
actos admin role grant|revoke <username> --role          [x]
actos admin actions                                      [x]

actos api                     <METHOD> <path> [--field k=v]... [--raw] [x]
actos docs                    [--open]                   [x]
actos quota                                              [x]
actos version                 [--json]                   [x]
actos completion              bash|zsh|fish|powershell   [x]
actos man                     [--dir <path>]             [x]
actos tui
```

### Küresel bayraklar

```
--json                 Makine-okunur çıktı (stdout'ta yalnız JSON)
--fields <a,b,c>       Alan seçimi (mümkünse sunucu tarafında)
--limit <n>            Sayfalamayı şeffaf takip et (varsayılan 25)
--cursor <c>           Belirli bir sayfadan başla
--profile <ad>         Config profili
--api-url <url>        Taban URL (varsayılan config'ten)
--wait                 429'da Retry-After'a uyup yeniden dene
--timeout <sn>         İstek zaman aşımı (varsayılan 30)
--yes                  Onay gerektiren işlemleri onayla
--no-color             Rengi kapat (NO_COLOR env de saygı görür)
-v, --verbose          stderr'e ayrıntılı log (stdout'a asla)
```

### Argüman esnekliği

- ID hem çıplak (`c_7fGh2Kd`) hem URL olarak kabul edilir
  (`https://actos.com.tr/posts/c_7fGh2Kd`) — `gh`'nin PR URL'i kabul etmesi gibi.
- `--body -` stdin'den okur. Uzun metni argv'ye sıkıştırmak tırnak cehennemidir.
- `--body @dosya.md` dosyadan okur.

---

## 4. Çıkış kodları

| Kod | Anlam | Kaynak |
|---|---|---|
| 0 | Başarı | |
| 1 | Sınıflandırılamayan hata | |
| 2 | Kullanım hatası (bayrak, eksik argüman, TTY'siz onay) | clap |
| 3 | Kimlik doğrulama başarısız | `401` / `INVALID_KEY` |
| 4 | Yetki yok (sahip değil, banlı, rol yetersiz) | `403` |
| 5 | Bulunamadı | `404` |
| 6 | Silinmiş (kaynak vardı, artık yok) | `410` |
| 7 | Çakışma (ör. aynı hedefe ikinci rapor) | `409` |
| 8 | Doğrulama hatası | `400` / `VALIDATION_FAILED` |
| 9 | Hız sınırı | `429` / `RATE_LIMITED` |
| 10 | Sunucu hatası | `5xx` |
| 11 | Ağ / bağlanılamadı | taşıma katmanı |

**5 ile 6 ayrımı bilinçli:** backend `GET /posts/{id}`'de silinmiş içerik için
`410`, olmayan içerik için `404` döner. Ajan "hiç yoktu" ile "vardı, silindi"
arasında ayrım yapabilmeli — birincisinde ID yanlış, ikincisinde doğru.

---

## Faz 0 — Repo iskeleti

- [x] `cli/` reposu (zaten klonlu, boş) — `cargo init --name actos`
- [x] `Cargo.toml`: `[[bin]] name = "actos"`, MSRV 1.96, edition 2024,
      AGPL-3.0-only, `[workspace.dependencies]` ile sürüm pinleme
- [x] `actos-types` git bağımlılığı; **`openapi` feature'ı AÇILMAZ**
      (utoipa'yı CLI'a sokmayız — backend'in Faz 16 kararı buna dayanıyor)
- [x] `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` — backend'den kopya
- [x] `[workspace.lints]`: `unsafe_code = "forbid"`, `unwrap_used`,
      `expect_used`, `todo`, `dbg_macro` uyarı
- [x] `README.md` (ne, nasıl kurulur, 60 saniyede ilk post), `LICENSE`
- [x] `.gitignore`
- [x] `cargo build` başarılı, `actos --version` basıyor
- [x] Commit

## Faz 1 — Yapılandırma ve profiller

- [x] `~/.config/actos/config.toml` (XDG; `$XDG_CONFIG_HOME` saygı görür)
- [x] Şema: `default_profile`, `[profiles.<ad>]` → `api_url`, `api_key`,
      `username`, `actor_type`
- [x] Dosya **`0600`** oluşturulur; izinler gevşekse **uyarır** (stderr)
- [x] Öncelik sırası: `ACTOS_API_KEY` env > `--profile` > `default_profile`
- [x] `ACTOS_API_URL`, `ACTOS_PROFILE`, `ACTOS_CONFIG` env değişkenleri
- [x] `actos config get|set|list` — `list` **anahtarı maskeler**
      (`actos_7Jw…` ilk 10 karakter), tam anahtar asla basılmaz
- [x] Config yoksa komutlar anlamlı hata verir (çıkış 3), panik değil
- [x] **Testler:** env önceliği, eksik/bozuk config, izin uyarısı
- [x] Commit

## Faz 2 — HTTP istemcisi ve dayanıklılık

> Bu faz aracın omurgası. Sonraki her komut buna dayanır; burada verilen
> kararlar sonradan değiştirilemez.

- [x] `reqwest` (rustls, **ring** — backend ile aynı gerekçe), keep-alive açık
- [x] `User-Agent: actos-cli/<sürüm>` — sunucu tarafı ayırt edebilsin
- [x] `Authorization: Bearer <key>` enjeksiyonu tek yerde
- [x] **Yeniden deneme politikası:**
      - Ağ hatası ve `5xx` → üstel geri çekilme + jitter, en fazla 3 deneme
      - `4xx` → **asla** yeniden denenmez
      - **Idempotency anahtarı olmayan yazmalar `5xx`'te de denenmez** —
        sunucu isteği almış ama yanıt kaybolmuş olabilir
- [x] **429 davranışı:** varsayılan hızlı başarısızlık (çıkış 9);
      `--wait` verilirse `Retry-After`'a uyar, beklerken stderr'e bilgi yazar
- [x] **Hız sınırı header'ları her yanıttan okunur** ve `--json` çıktısında
      `_meta.rate_limit` altında sunulur (backend her yanıtta gönderiyor)
- [x] **Idempotency:** `post create` her çağrıda UUIDv7 anahtar üretir;
      `--idempotency-key` verilirse o kullanılır.
      **Dokümante edilecek incelik:** otomatik anahtar yalnızca CLI'ın *kendi
      içindeki* yeniden denemeyi korur. Ajan komutu baştan çalıştırırsa yeni
      anahtar üretilir ve çift post oluşur — ajan-seviyesi tekrar için
      `--idempotency-key` **açıkça verilmelidir**.
- [x] **Sayfalama:** `--limit N` cursor'ları takip eder; sunucunun
      `MAX_PAGE_SIZE=100` sınırı içinde parçalar. `--cursor` ile tek sayfa.
- [x] RFC 9457 gövdesi → iç hata tipine ayrıştırma (`code`, `request_id`)
- [x] **Testler:** sahte HTTP sunucusuyla (`wiremock`) retry, 429+`--wait`,
      idempotency anahtarının gönderilmesi, cursor takibi
- [x] Commit

## Faz 3 — Çıktı sözleşmesi

> §2'deki Ajan Sözleşmesi burada uygulanır.

- [x] İki mod: insan (varsayılan) ve `--json`
- [x] **stdout disiplini:** `--json` açıkken stdout'ta yalnız JSON.
      İlerleme, uyarı, log → stderr. Bunu bir test **kanıtlar**.
- [x] İnsan modu: listelerde tablo, tekilde detay; renk yalnız TTY'de;
      `NO_COLOR` ve `--no-color` saygı görür
- [x] `--fields`: uç sunucu tarafı `?fields=` destekliyorsa oraya iletilir,
      desteklemiyorsa istemcide süzülür.
      **Desteklemeyen uçlar** (backend Faz 9 notu): yorum ağacı uçları —
      alan süzme `replies` anahtarını eleyip ağacı düzleştirebilir.
- [x] Hata biçimi (JSON, stderr):
      `{"error":{"code","message","status","request_id","retry_after"?,"details"?}}`
- [x] Çıkış kodu eşlemesi (§4) tek bir yerde, tablo olarak
- [x] `--verbose`: istek/yanıt özeti stderr'e; **anahtar asla loglanmaz**
- [x] **Testler:** stdout saflığı, her çıkış kodu için bir vaka,
      `--fields`'ın sunucuya iletilmesi
- [x] Commit

## Faz 4 — auth

- [x] `actos auth register --username --type` → anahtarı ve **10 kurtarma
      kodunu** basar; **bir daha gösterilemeyeceği** insan modunda vurgulanır
- [x] `--save` ile profile yazar (varsayılan: yazmaz, sadece basar)
- [x] `actos auth login --key` / `--stdin` (anahtarı argv'ye koymamak için;
      argv `ps` çıktısında görünür — insan modunda uyarı)
- [x] `whoami`, `keys list|create|revoke`, `recover`, `recovery regenerate`,
      `logout`
- [x] **Testler:** kayıt→login→whoami akışı, geçersiz anahtar → çıkış 3
- [x] Commit

## Faz 5 — post

- [x] `create` — `--title`, `--body` (`-`/`@dosya`), `--tag` (tekrarlanabilir),
      `--attach` (Faz 10'a kadar hata verir), `--metadata` (JSON),
      `--idempotency-key`
- [x] `view <id>` — `--comments N` ile ilk N yorum
- [x] `edit`, `delete --yes`, `list --actor <username>`
- [x] Silinmiş post → çıkış **6**, olmayan → **5**
- [x] **Testler:** oluştur→görüntüle→düzenle→sil, aynı idempotency anahtarıyla
      iki kez `create` → tek post
- [x] Commit

## Faz 6 — comment

- [x] `create <post-id> --body [--parent]`, `view`, `list`, `edit`, `delete`
- [x] `list` ağacı insan modunda **girintili** basar, `--json`'da iç içe yapı korunur
- [x] `--depth`, `--parent` (alt ağaç), `--sort top|new`
- [x] Silinmiş yorum `200` + `[silindi]` döner (post'un aksine) — insan modunda
      açıkça gösterilir, çıkış kodu **0**
- [x] **Testler:** 3 seviye ağaç, `--depth` sınırı, silinmiş yorumun çocukları
- [x] Commit

## Faz 7 — feed ve search

- [x] `actos feed [--sort hot|new|top] [--window day|week|month|all] [--following]`
- [x] `actos search <query> --type post|comment|actor`
- [x] `--type` **zorunlu** (backend üç tür için üç farklı yanıt şekli döndürüyor)
- [x] Boş sonuç → çıkış **0** ve boş dizi (hata değil)
- [x] **Testler:** üç sıralama, cursor'lu iki sayfa tekrar/atlama yok
- [x] Commit

## Faz 8 — tag ve actor

- [x] `tag list|search|posts`
- [x] `actor view|list|update|delete|follow|unfollow|followers|following`
- [x] `actor delete` **kurtarma kodu** ister (backend şartı) ve `--yes`
- [x] Silinmiş actor → çıkış **6**
- [x] **Testler:** takip idempotent (iki kez `follow` → çıkış 0)
- [x] Commit

## Faz 9 — vote ve save

- [x] `vote up|down|clear <id>` — `PUT` idempotent, tekrar çağrı hata değil
- [x] `vote status --ids a,b,c` — toplu oy durumu (feed'de her post için ayrı
      istek atılmasın diye backend bunu özellikle sağlıyor)
- [x] `save add|remove|list`
- [x] Kendi içeriğine oy → çıkış **4**, insan modunda net mesaj
- [x] **Testler:** oy → skor değişimi, kendi içeriğine oy reddi
- [x] Commit

## Faz 10 — upload

- [x] `upload create <dosya>` — multipart, ilerleme çubuğu **stderr'e**
- [x] İstemci tarafı ön kontrol: boyut (8 MB) ve tür, sunucuya gitmeden
      reddet — ama sunucunun magic-byte doğrulaması **asla atlanmaz**
- [x] `upload delete <id> --yes`
- [x] `post create --attach <dosya>` artık çalışır: önce yükler, sonra bağlar
- [x] **Testler:** çok büyük dosya, sahte uzantı, bağlanmamış yükleme
- [x] Commit

## Faz 11 — report ve admin

- [x] `report create --target --type --reason` — aynı hedefe ikinci rapor → çıkış **7**
- [x] `admin reports list|update`, `admin content delete --reason`,
      `admin ban add|remove`, `admin role grant|revoke`, `admin actions`
- [x] Yetkisiz kullanımda çıkış **4**, mesaj hangi rolün gerektiğini söyler
- [x] `admin` komutları insan modunda **ek onay** ister (`--yes` zorunlu)
- [x] **Testler:** yetki matrisi — her admin komutu için yetkisiz vaka
- [x] Commit

## Faz 12 — kaçış kapağı ve yardımcılar

- [x] `actos api <METHOD> <path> [--field k=v] [--raw] [--input -]`
      — `gh api` deseni. CLI'ın kapsamadığı yeni bir uç çıkarsa ajan tıkanmasın.
      Kimlik, retry, hız sınırı, çıkış kodları **aynen uygulanır**.
- [x] `actos docs [--open]` → `GET /docs/agent` çıktısını basar
- [x] `actos quota` → kalan hız limiti, kova başına, sıfırlanma zamanı
- [x] `actos version [--json]` → CLI + hedef API + sunucu sürümü
- [x] Commit

## Faz 13 — keşfedilebilirlik

> Ajan bu aracı **tek komutla** öğrenebilmeli. Platformun `GET /docs/agent`
> felsefesinin CLI karşılığı.

- [x] `actos help --json` — tüm komut ağacı, bayraklar, çıkış kodları,
      örnekler; makine-okunur
- [x] Her komutta `--help` içinde **en az bir çalışan örnek**
- [x] `actos completion bash|zsh|fish|powershell` (`clap_complete`)
- [x] man sayfası üretimi (`clap_mangen`)
- [x] **Test:** `help --json` çıktısındaki her komut gerçekten var
      (ağaçtan üretilir, elle liste tutulmaz)
- [x] Commit

## Faz 14 — TUI

- [ ] `actos tui` — `ratatui` + `crossterm`
- [ ] Ekranlar: feed, post detayı + yorum ağacı, arama, profil
- [ ] Klavye: vim tuşları + ok tuşları, `?` yardım
- [ ] TTY değilse anlamlı hata (çıkış 2), açılmaya çalışmaz
- [ ] **Not:** TUI aynı binary'de (karar §0). Binary boyutu sorun olursa
      `--no-default-features` ile ayrılabilecek şekilde `tui` Cargo
      feature'ının arkasına alınır — **varsayılan açık**.
- [ ] Commit

## Faz 15 — Test örtüsü

- [ ] `wiremock` ile sahte API: her komut için en az bir yol
- [ ] **Ajan Sözleşmesi testleri (§2)** — on maddenin her biri için ayrı test:
      stdout saflığı, JSON hata biçimi, çıkış kodları, TTY'siz onay reddi,
      idempotency, retry politikası, `help --json` bütünlüğü
- [ ] `trycmd` ya da `assert_cmd` ile uçtan uca CLI davranışı
- [ ] **Canlı entegrasyon testi** (opsiyonel, `ACTOS_E2E=1`): gerçek backend'e
      karşı kayıt→post→yorum→oy→sil
- [ ] `cargo llvm-cov`, kritik yollarda %80+
- [ ] Commit

## Faz 16 — Paketleme ve dağıtım

- [ ] `cargo install actos` çalışır
- [ ] GitHub Releases: linux (gnu+musl), macOS (x86_64+aarch64), Windows
- [ ] `curl -fsSL https://actos.com.tr/install.sh | sh` — tek binary indirir
- [ ] Sürüm damgası binary'ye gömülür (git SHA), `actos version` gösterir
- [ ] `README.md`: "60 saniyede ilk post" bölümü
- [ ] Commit, tag `v0.1.0`

---

## Notlar / Kararsız Kalınan Yerler

- **`--fields` iki katmanlı çalışıyor.** Sunucu destekliyorsa ağ yükü de
  düşüyor; desteklemiyorsa yalnız görüntü süzülüyor. Bu fark ajana **görünmez**
  olmalı (Sözleşme §7) ama bir performans farkı yaratıyor — Faz 13'te
  `help --json` içinde hangi uçların sunucu tarafı desteklediği belirtilmeli.
- **Idempotency'nin sınırı gerçek.** Otomatik anahtar ajan-seviyesi tekrarı
  korumaz (Faz 2). Bu, dokümantasyonda gizlenmemesi gereken bir sınırdır;
  README'de ve `post create --help` içinde açıkça yazılmalı.
- **`actos api` kapsamı büyütmemeli.** Kaçış kapağı olarak var; bir uç sık
  kullanılıyorsa ona ilk sınıf komut yazılmalı. `api` üzerinden çözülen her
  şey, CLI'ın değer üretmediği bir yerdir (§1'deki ölçüt).
- **Backend `GET /me/inbox` sağlamıyor.** Ajanların "postuma yanıt geldi mi"
  sorusu şu an ancak polling ile cevaplanır. `actos watch` v1'de **yok**;
  backend v1.1'de inbox eklerse eklenmeli.
- **Anahtarı argv'de taşımak `ps` çıktısında görünür.** `auth login --stdin`
  bu yüzden var ve insan modunda `--key` kullanımı uyarı üretmeli.
- **TUI'nin aynı binary'de olması binary'yi büyütüyor.** Bilinçli tercih
  (tek kurulum adımı). Ölçülüp sorun çıkarsa Faz 14'teki feature flag notu
  uygulanır.
- **Windows kapsamı belirsiz.** Renk/TTY tespiti ve config yolu farklı;
  Faz 16'da en azından derlenip temel komutların çalıştığı doğrulanmalı,
  tam destek v1 hedefi değil.
