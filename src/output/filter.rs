use serde_json::{Map, Value};

/// JSON çıktısını istenen alanlara (`--fields a,b,c`) göre süzer (`PLAN.md` Faz 3).
///
/// Nesnelerde yalnızca belirtilen anahtarları tutar. Eğer nesne bir sayfalanmış zarf ise
/// (içinde `"posts"`, `"items"` gibi bir dizi barındırıyorsa), dizinin her bir öğesine de
/// alan süzme uygular.
#[must_use]
pub fn filter_fields(val: &Value, fields: &[String]) -> Value {
    if fields.is_empty() {
        return val.clone();
    }

    match val {
        Value::Object(map) => {
            let mut filtered = Map::new();

            // 1. Zarf kontrolü: bilinen liste alanları var mı?
            let candidate_keys = [
                "items", "posts", "comments", "actors", "tags", "reports", "keys", "votes", "saved",
            ];

            let mut has_envelope_array = false;
            for key in candidate_keys {
                if let Some(Value::Array(arr)) = map.get(key) {
                    has_envelope_array = true;
                    let filtered_arr: Vec<Value> =
                        arr.iter().map(|item| filter_fields(item, fields)).collect();
                    filtered.insert(key.to_string(), Value::Array(filtered_arr));
                }
            }

            if has_envelope_array {
                // Diğer zarf alanlarını (ör. "next_cursor") koru
                for (k, v) in map {
                    if !candidate_keys.contains(&k.as_str()) {
                        filtered.insert(k.clone(), v.clone());
                    }
                }
                Value::Object(filtered)
            } else {
                // Düz nesne: yalnızca fields içinde yer alan anahtarları tut
                for field in fields {
                    if let Some(v) = map.get(field) {
                        filtered.insert(field.clone(), v.clone());
                    }
                }
                Value::Object(filtered)
            }
        }

        Value::Array(arr) => {
            let filtered_arr: Vec<Value> =
                arr.iter().map(|item| filter_fields(item, fields)).collect();
            Value::Array(filtered_arr)
        }

        scalar => scalar.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_object_fields() {
        let val = json!({
            "id": "post_1",
            "title": "Başlık",
            "body": "Gövde metni",
            "author": "alice"
        });

        let fields = vec!["id".to_string(), "title".to_string()];
        let filtered = filter_fields(&val, &fields);

        assert_eq!(filtered, json!({ "id": "post_1", "title": "Başlık" }));
    }

    #[test]
    fn test_filter_array_fields() {
        let val = json!([
            { "id": 1, "name": "A", "extra": "foo" },
            { "id": 2, "name": "B", "extra": "bar" }
        ]);

        let fields = vec!["id".to_string(), "name".to_string()];
        let filtered = filter_fields(&val, &fields);

        assert_eq!(
            filtered,
            json!([
                { "id": 1, "name": "A" },
                { "id": 2, "name": "B" }
            ])
        );
    }

    #[test]
    fn test_filter_envelope_fields() {
        let val = json!({
            "posts": [
                { "id": "p1", "title": "T1", "body": "B1" },
                { "id": "p2", "title": "T2", "body": "B2" }
            ],
            "next_cursor": "cur_next"
        });

        let fields = vec!["id".to_string(), "title".to_string()];
        let filtered = filter_fields(&val, &fields);

        assert_eq!(
            filtered,
            json!({
                "posts": [
                    { "id": "p1", "title": "T1" },
                    { "id": "p2", "title": "T2" }
                ],
                "next_cursor": "cur_next"
            })
        );
    }
}
