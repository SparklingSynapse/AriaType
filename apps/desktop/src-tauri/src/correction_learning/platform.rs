#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", test))]
const MAX_TEXT_SNAPSHOT_CHARS: i32 = 50_000;

pub async fn read_focused_editable_text() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        tokio::task::spawn_blocking(macos::read_focused_editable_text)
            .await
            .ok()
            .flatten()
    }

    #[cfg(target_os = "windows")]
    {
        tokio::task::spawn_blocking(windows::read_focused_editable_text)
            .await
            .ok()
            .flatten()
    }

    #[cfg(target_os = "linux")]
    {
        linux::read_focused_editable_text().await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

fn non_empty_text(text: String) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(any(target_os = "linux", test))]
fn bounded_text_range(character_count: i32, caret_offset: Option<i32>) -> Option<(i32, i32)> {
    if character_count <= 0 {
        return None;
    }

    if character_count <= MAX_TEXT_SNAPSHOT_CHARS {
        return Some((0, character_count));
    }

    let half_window = MAX_TEXT_SNAPSHOT_CHARS / 2;
    let caret = caret_offset
        .unwrap_or(character_count)
        .clamp(0, character_count);
    let mut start = caret.saturating_sub(half_window);
    let mut end = (start + MAX_TEXT_SNAPSHOT_CHARS).min(character_count);
    start = end.saturating_sub(MAX_TEXT_SNAPSHOT_CHARS);
    end = (start + MAX_TEXT_SNAPSHOT_CHARS).min(character_count);

    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::{bounded_text_range, non_empty_text, MAX_TEXT_SNAPSHOT_CHARS};

    #[test]
    fn keeps_non_blank_text_unchanged() {
        assert_eq!(
            non_empty_text("  hello  ".to_string()).as_deref(),
            Some("  hello  ")
        );
    }

    #[test]
    fn rejects_blank_text() {
        assert_eq!(non_empty_text(" \n\t ".to_string()), None);
    }

    #[test]
    fn reads_complete_short_text_range() {
        assert_eq!(bounded_text_range(42, Some(12)), Some((0, 42)));
    }

    #[test]
    fn bounds_large_text_around_caret() {
        assert_eq!(
            bounded_text_range(120_000, Some(80_000)),
            Some((
                80_000 - MAX_TEXT_SNAPSHOT_CHARS / 2,
                80_000 + MAX_TEXT_SNAPSHOT_CHARS / 2
            ))
        );
    }

    #[test]
    fn bounds_large_text_near_document_end() {
        assert_eq!(
            bounded_text_range(120_000, Some(119_990)),
            Some((120_000 - MAX_TEXT_SNAPSHOT_CHARS, 120_000))
        );
    }
}
