use serde_json::Value;

/// Sunucu tarafı sayfa başına azami öğe sınırı (`PLAN.md` Faz 2).
pub const MAX_PAGE_SIZE: u32 = 100;

/// Sayfalanmış yanıttan bir sonraki sayfa imlecini (`next_cursor`) okur.
#[must_use]
pub fn extract_next_cursor(val: &Value) -> Option<String> {
    val.get("next_cursor")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
}

/// Sayfalanmış JSON yanıtındaki ana dizi alan adını bulur (ör. `"posts"`, `"comments"`, `"items"`).
#[must_use]
pub fn find_array_field_name(val: &Value) -> Option<String> {
    if let Some(map) = val.as_object() {
        let candidate_keys = [
            "items", "posts", "comments", "actors", "tags", "reports", "keys", "votes", "saved",
        ];

        for key in candidate_keys {
            if let Some(Value::Array(_)) = map.get(key) {
                return Some(key.to_string());
            }
        }

        for (k, v) in map {
            if let Value::Array(_) = v {
                return Some(k.clone());
            }
        }
    }
    None
}

/// İki sayfalanmış yanıtı şeffaf biçimde birleştirir.
///
/// `target_limit` öğe sayısına ulaşıldığında eklemeyi durdurur.
pub fn merge_page(
    accumulated: &mut Value,
    page: Value,
    target_limit: usize,
) -> (usize, Option<String>) {
    let next_cursor = extract_next_cursor(&page);

    if accumulated.is_null() {
        *accumulated = page;
        if let Some(key) = find_array_field_name(accumulated)
            && let Some(Value::Array(arr)) = accumulated.get_mut(&key)
        {
            if arr.len() > target_limit {
                arr.truncate(target_limit);
            }
            return (arr.len(), next_cursor);
        }
        return (0, next_cursor);
    }

    // İkinci veya sonraki sayfaları birleştir
    let page_items = if let Some(key) = find_array_field_name(&page)
        && let Some(Value::Array(arr)) = page.get(&key)
    {
        Some(arr.clone())
    } else {
        None
    };

    if let Some(key) = find_array_field_name(accumulated)
        && let Some(new_items) = page_items
        && let Some(Value::Array(acc_arr)) = accumulated.get_mut(&key)
    {
        let remaining_needed = target_limit.saturating_sub(acc_arr.len());
        let to_add = new_items.into_iter().take(remaining_needed);
        acc_arr.extend(to_add);
    }

    if let Some(map) = accumulated.as_object_mut() {
        if let Some(cur) = &next_cursor {
            map.insert("next_cursor".to_string(), Value::String(cur.clone()));
        } else {
            map.insert("next_cursor".to_string(), Value::Null);
        }
    }

    let current_count = find_array_field_name(accumulated)
        .and_then(|k| accumulated.get(&k))
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);

    (current_count, next_cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_next_cursor() {
        let val = json!({ "next_cursor": "cursor_123" });
        assert_eq!(extract_next_cursor(&val), Some("cursor_123".to_string()));

        let empty = json!({ "next_cursor": null });
        assert_eq!(extract_next_cursor(&empty), None);
    }

    #[test]
    fn test_merge_page() {
        let page1 = json!({
            "posts": [{"id": 1}, {"id": 2}],
            "next_cursor": "c2"
        });
        let page2 = json!({
            "posts": [{"id": 3}, {"id": 4}],
            "next_cursor": null
        });

        let mut acc = Value::Null;
        let (count1, cursor1) = merge_page(&mut acc, page1, 3);
        assert_eq!(count1, 2);
        assert_eq!(cursor1, Some("c2".to_string()));

        let (count2, cursor2) = merge_page(&mut acc, page2, 3);
        assert_eq!(count2, 3);
        assert_eq!(cursor2, None);

        let posts = acc["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 3);
        assert_eq!(posts[0]["id"], 1);
        assert_eq!(posts[1]["id"], 2);
        assert_eq!(posts[2]["id"], 3);
    }
}
