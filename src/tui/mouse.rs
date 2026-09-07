//! Fare desteği: tıklanabilir alan kaydı ve olay eşleme (TUI F0).
//!
//! Render sırasında her tıklanabilir bölge `hit_areas`'a kaydedilir;
//! fare olayı geldiğinde [`hit_test`] ile hangi aksiyonun tetikleneceği
//! bulunur. Saf fonksiyonlar burada tutulur ki birim testle kilitlensin.

use ratatui::layout::Rect;

use crate::tui::app::CurrentTab;

/// Fareyle tetiklenebilen aksiyonlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    /// Sekmeye geç.
    SwitchTab(CurrentTab),
    /// Feed satırını seç (zaten seçiliyse aç).
    SelectFeed(usize),
    /// Arama sonucu satırını seç.
    SelectSearch(usize),
    /// Tag satırını seç (zaten seçiliyse postları aç).
    SelectTag(usize),
    /// Tag postu satırını seç (zaten seçiliyse detayı aç).
    SelectTagPost(usize),
    /// Aktör satırını seç.
    SelectActor(usize),
    /// Bildirim satırını seç.
    SelectNotif(usize),
    /// Kayıtlı içerik satırını seç (zaten seçiliyse aç).
    SelectSave(usize),
}

/// (x, y) hücresine denk gelen aksiyonu bulur.
///
/// Alanlar çakışırsa ilk kaydedilen kazanır.
#[must_use]
pub fn hit_test(
    areas: &[(Rect, MouseAction)],
    x: u16,
    y: u16,
) -> Option<MouseAction> {
    areas
        .iter()
        .find(|(rect, _)| {
            x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height)
        })
        .map(|(_, action)| *action)
}

/// Sekme çubuğu düzenini hesaplar: her sekmenin metin genişliğinden
/// tıklama dikdörtgenlerini üretir.
///
/// `origin_x` çubuğun iç sol kenarı, `row_y` yazı satırıdır. Sekmeler
/// arası iki hücre boşluk bırakılır.
#[must_use]
pub fn tab_bar_areas(
    labels: &[&str],
    origin_x: u16,
    row_y: u16,
) -> Vec<Rect> {
    let mut areas = Vec::with_capacity(labels.len());
    let mut x = origin_x;
    for label in labels {
        let w = label.chars().count() as u16;
        areas.push(Rect::new(x, row_y, w, 1));
        x = x.saturating_add(w).saturating_add(2);
    }
    areas
}

/// Liste satırlarının tıklama alanları: çerçeveli listenin içi,
/// satır başına bir hücre. `count` kadar (görünürle sınırlı) üretir.
#[must_use]
pub fn list_row_areas(list_area: Rect, count: usize) -> Vec<Rect> {
    let visible = (list_area.height.saturating_sub(2)) as usize;
    let top = list_area.y.saturating_add(1);
    (0..count.min(visible))
        .map(|i| {
            Rect::new(
                list_area.x.saturating_add(1),
                top.saturating_add(i as u16),
                list_area.width.saturating_sub(2),
                1,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn test_hit_test_inside_and_edges() {
        let areas = vec![
            (rect(5, 2, 10, 1), MouseAction::SelectFeed(0)),
            (rect(5, 3, 10, 1), MouseAction::SelectFeed(1)),
        ];
        assert_eq!(
            hit_test(&areas, 5, 2),
            Some(MouseAction::SelectFeed(0))
        );
        assert_eq!(
            hit_test(&areas, 14, 3),
            Some(MouseAction::SelectFeed(1))
        );
        // Sağ/sol kenar dışarıda.
        assert_eq!(hit_test(&areas, 15, 2), None);
        assert_eq!(hit_test(&areas, 4, 2), None);
        assert_eq!(hit_test(&areas, 0, 0), None);
    }

    #[test]
    fn test_hit_test_first_area_wins_overlap() {
        let areas = vec![
            (rect(0, 0, 10, 10), MouseAction::SelectFeed(0)),
            (rect(2, 2, 10, 10), MouseAction::SelectFeed(1)),
        ];
        assert_eq!(hit_test(&areas, 3, 3), Some(MouseAction::SelectFeed(0)));
    }

    #[test]
    fn test_tab_bar_areas_layout() {
        let areas = tab_bar_areas(&[" [F1] Feed ", " [F2] Search "], 1, 1);
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0], rect(1, 1, 11, 1));
        assert_eq!(areas[1], rect(14, 1, 13, 1));
        // İkinci sekmenin içine düşen tıklama ikinci alanı bulur.
        let actions: Vec<(Rect, MouseAction)> = areas
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let tab = if i == 0 {
                    CurrentTab::Feed
                } else {
                    CurrentTab::Search
                };
                (r, MouseAction::SwitchTab(tab))
            })
            .collect();
        assert_eq!(
            hit_test(&actions, 16, 1),
            Some(MouseAction::SwitchTab(CurrentTab::Search))
        );
    }

    #[test]
    fn test_list_row_areas_visible_window() {
        // 10 satırlık alan: başlık+çerçeve düşer, 8 satır görünür.
        let area = Rect::new(0, 0, 40, 10);
        let rows = list_row_areas(area, 20);
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0], Rect::new(1, 1, 38, 1));
        assert_eq!(rows[7], Rect::new(1, 8, 38, 1));
        // Az öğe: öğe sayısı kadar.
        assert_eq!(list_row_areas(area, 3).len(), 3);
    }
}
