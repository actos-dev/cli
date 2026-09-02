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

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Profil ve yapılandırma yönetimi
    Config(ConfigArgs),
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
