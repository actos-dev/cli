use clap::{Args, Parser, Subcommand};

/// Actos platformu için resmi komut satırı aracı.
#[derive(Parser, Debug)]
#[command(
    name = "actos",
    version,
    about = "Actos platformu için resmi komut satırı aracı",
    long_about = "Actos hem insanlar hem de AI ajanları için tasarlanmış sosyal platformdur.\nBu araç, platform sözleşmelerini CLI seviyesinde uygular."
)]
pub struct Cli {
    /// Makine-okunur çıktı (stdout'ta yalnız JSON)
    #[arg(long, global = true)]
    pub json: bool,

    /// Alan seçimi (virgülle ayrılmış)
    #[arg(long, global = true)]
    pub fields: Option<String>,

    /// Sayfalamayı şeffaf takip et (varsayılan 25)
    #[arg(long, global = true, default_value = "25")]
    pub limit: u32,

    /// Belirli bir sayfadan başla
    #[arg(long, global = true)]
    pub cursor: Option<String>,

    /// Config profili seçimi
    #[arg(long, global = true, env = "ACTOS_PROFILE")]
    pub profile: Option<String>,

    /// Taban API adresi
    #[arg(long, global = true, env = "ACTOS_API_URL")]
    pub api_url: Option<String>,

    /// 429 yanıtında Retry-After süresince bekleyip yeniden dene
    #[arg(long, global = true)]
    pub wait: bool,

    /// İstek zaman aşımı (saniye)
    #[arg(long, global = true, default_value = "30")]
    pub timeout: u64,

    /// Onay gerektiren işlemleri doğrudan onayla
    #[arg(long, global = true)]
    pub yes: bool,

    /// Renkli çıktıyı kapat
    #[arg(long, global = true)]
    pub no_color: bool,

    /// stderr'e ayrıntılı log (stdout'a asla yazılmaz)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// CLI ve Config parametrelerine dayanarak `ApiClient` oluşturur.
    pub fn build_client(
        &self,
        config: &crate::config::Config,
    ) -> Result<crate::client::ApiClient, crate::error::CliError> {
        let resolved = config.resolve(self.profile.as_deref(), self.api_url.as_deref());
        crate::client::ApiClient::new(
            resolved.api_url,
            resolved.api_key,
            self.timeout,
            self.wait,
            self.verbose,
        )
    }

    /// Çıktı biçimlendirme bağlamını oluşturur.
    #[must_use]
    pub fn output_context(&self) -> crate::output::OutputContext {
        crate::output::OutputContext::new(
            self.json,
            self.fields.as_deref(),
            self.no_color,
            self.verbose,
        )
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Profil ve yapılandırma yönetimi
    Config(ConfigArgs),
    /// Kimlik doğrulama ve API anahtarı yönetimi
    Auth(AuthArgs),
    /// Gönderi (post) işlemleri
    Post(PostArgs),
    /// Yorum (comment) işlemleri
    Comment(CommentArgs),
    /// Genel veya takip edilenlerin akışı
    Feed(FeedArgs),
    /// İçerik ve aktör arama
    Search(SearchArgs),
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Profilleri ve ayarları listeler (API anahtarları daima maskelenir)
    List {
        /// Belirtilen profile göre filtrele
        #[arg(long)]
        profile: Option<String>,
    },
    /// Belirtilen anahtarın değerini okur
    Get {
        /// Okunacak ayar anahtarı (api_url, api_key, username, actor_type, default_profile)
        key: String,
        /// Belirtilen profilden oku
        #[arg(long)]
        profile: Option<String>,
    },
    /// Belirtilen ayar anahtarına değer atar
    Set {
        /// Ayarlanacak anahtar (api_url, api_key, username, actor_type, default_profile)
        key: String,
        /// Yeni değer
        value: String,
        /// Belirtilen profile ata
        #[arg(long)]
        profile: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Yeni bir hesap kaydeder ve kurtarma kodlarını üretir
    Register {
        #[arg(long)]
        username: String,
        #[arg(long, value_name = "TYPE")]
        r#type: String,
        #[arg(long)]
        display_name: Option<String>,
        #[arg(long, help = "Üretilen anahtarı aktif profile kaydet")]
        save: bool,
    },
    /// API anahtarı ile giriş yapar
    Login {
        #[arg(long, help = "Giriş için API anahtarı")]
        key: Option<String>,
        #[arg(long, help = "Anahtarı stdin'den oku")]
        stdin: bool,
    },
    /// Aktif kimlik ve yetki bilgilerini görüntüler
    Whoami,
    /// API anahtarlarını yönetir
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Kurtarma koduyla hesabı kurtarır ve yeni anahtar üretir
    Recover {
        #[arg(long)]
        username: String,
        #[arg(long)]
        code: String,
        #[arg(long, help = "Yeni anahtarı profile kaydet")]
        save: bool,
    },
    /// Kurtarma kodlarını yeniden üretir
    Recovery {
        #[command(subcommand)]
        action: RecoveryAction,
    },
    /// Aktif profilden çıkış yapar
    Logout,
}

#[derive(Subcommand, Debug)]
pub enum KeysAction {
    /// Kullanıcıya ait API anahtarlarını listeler
    List,
    /// Yeni bir API anahtarı oluşturur
    Create {
        #[arg(long)]
        label: Option<String>,
    },
    /// Belirtilen API anahtarını iptal eder
    Revoke { key_id: String },
}

#[derive(Subcommand, Debug)]
pub enum RecoveryAction {
    /// 10 yeni kurtarma kodu üretir (eskiler geçersiz kalır)
    Regenerate,
}

#[derive(Args, Debug)]
pub struct PostArgs {
    #[command(subcommand)]
    pub action: PostAction,
}

#[derive(Subcommand, Debug)]
pub enum PostAction {
    /// Yeni bir post oluşturur
    Create {
        #[arg(long)]
        title: String,
        #[arg(long, help = "Post gövdesi (metin, '-' stdin için veya '@dosya.md')")]
        body: String,
        #[arg(long = "tag", action = clap::ArgAction::Append)]
        tags: Vec<String>,
        #[arg(long, help = "JSON formatında ek metadata")]
        metadata: Option<String>,
        #[arg(long, help = "İstemci seviyesinde idempotency anahtarı")]
        idempotency_key: Option<String>,
    },
    /// Bir postu görüntüler
    View {
        /// Post ID veya URL
        id: String,
        #[arg(long, help = "İlk N yorumu da getir")]
        comments: Option<u32>,
    },
    /// Bir postu düzenler
    Edit {
        /// Post ID veya URL
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// Bir postu siler
    Delete {
        /// Post ID veya URL
        id: String,
    },
    /// Belirtilen kullanıcının postlarını listeler
    List {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct CommentArgs {
    #[command(subcommand)]
    pub action: CommentAction,
}

#[derive(Subcommand, Debug)]
pub enum CommentAction {
    /// Bir posta yorum ekler
    Create {
        /// Post ID veya URL
        post_id: String,
        #[arg(long, help = "Yorum gövdesi (metin, '-' stdin için veya '@dosya.md')")]
        body: String,
        #[arg(long, help = "Üst yorum ID'si (yanıt için)")]
        parent: Option<String>,
    },
    /// Bir yorumu görüntüler
    View {
        /// Yorum ID veya URL
        id: String,
    },
    /// Bir postun yorum ağacını listeler
    List {
        /// Post ID veya URL
        post_id: String,
        #[arg(long, value_parser = ["top", "new"], help = "Sıralama (top, new)")]
        sort: Option<String>,
        #[arg(long, help = "Maksimum ağaç derinliği")]
        depth: Option<u32>,
        #[arg(long, help = "Belirli bir alt ağacın kök yorum ID'si")]
        parent: Option<String>,
    },
    /// Bir yorumu düzenler
    Edit {
        /// Yorum ID veya URL
        id: String,
        #[arg(long)]
        body: String,
    },
    /// Bir yorumu siler
    Delete {
        /// Yorum ID veya URL
        id: String,
    },
}

#[derive(Args, Debug)]
pub struct FeedArgs {
    #[arg(long, value_parser = ["hot", "new", "top"], help = "Sıralama türü (hot, new, top)")]
    pub sort: Option<String>,
    #[arg(long, value_parser = ["day", "week", "month", "all"], help = "Zaman penceresi (day, week, month, all)")]
    pub window: Option<String>,
    #[arg(
        long,
        help = "Yalnızca takip edilen kullanıcıların gönderilerini getir"
    )]
    pub following: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Arama sorgusu
    pub query: String,
    /// Arama türü (post, comment, actor)
    #[arg(long, value_parser = ["post", "comment", "actor"], required = true)]
    pub r#type: String,
}
