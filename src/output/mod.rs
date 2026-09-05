pub mod filter;

use crate::error::RateLimitInfo;
use serde_json::Value;

/// CLI çıktı bağlamı ve kuralları (`PLAN.md` Faz 3).
#[derive(Debug, Clone)]
pub struct OutputContext {
    pub json: bool,
    pub fields: Option<Vec<String>>,
    pub no_color: bool,
    pub verbose: bool,
}

impl OutputContext {
    /// Komut satırı bayraklarına göre yeni bir çıktı bağlamı oluşturur.
    #[must_use]
    pub fn new(json: bool, fields_str: Option<&str>, no_color_flag: bool, verbose: bool) -> Self {
        let fields = fields_str.map(|s| {
            s.split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        });

        let no_color = no_color_flag || std::env::var("NO_COLOR").is_ok();

        Self {
            json,
            fields,
            no_color,
            verbose,
        }
    }

    /// JSON çıktısını stdout'a yalnız ve yalnız saf JSON olarak basar (Ajan Sözleşmesi §2 kural 1).
    pub fn print_json(&self, val: &Value, rate_limit: Option<RateLimitInfo>) {
        let mut final_val = if let Some(ref f) = self.fields {
            filter::filter_fields(val, f)
        } else {
            val.clone()
        };

        // Hız sınırı meta bilgisini ekle (`PLAN.md` Faz 2 / Faz 3)
        if let Some(rl) = rate_limit
            && (rl.limit.is_some() || rl.remaining.is_some() || rl.reset.is_some())
            && let Some(map) = final_val.as_object_mut()
        {
            let mut meta = serde_json::Map::new();
            if let Ok(rl_val) = serde_json::to_value(rl) {
                meta.insert("rate_limit".to_string(), rl_val);
            }
            map.insert("_meta".to_string(), Value::Object(meta));
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&final_val).unwrap_or_default()
        );
    }
}

/// stderr'e uyarı basar.
pub fn warn(msg: &str) {
    eprintln!("warning: {msg}");
}

/// stderr'e bilgi mesajı basar.
pub fn info(msg: &str) {
    eprintln!("{msg}");
}
