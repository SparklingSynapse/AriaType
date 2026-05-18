use uiautomation::core::UIElement;
use uiautomation::patterns::{UITextPattern, UIValuePattern};
use uiautomation::UIAutomation;

pub fn read_focused_editable_text() -> Option<String> {
    let automation = UIAutomation::new().ok()?;
    let element = automation.get_focused_element().ok()?;

    read_text_pattern(&element)
        .or_else(|| read_value_pattern(&element))
        .or_else(|| element.get_name().ok().and_then(super::non_empty_text))
}

fn read_text_pattern(element: &UIElement) -> Option<String> {
    let pattern = element.get_pattern::<UITextPattern>().ok()?;
    let range = pattern.get_document_range().ok()?;
    range.get_text(-1).ok().and_then(super::non_empty_text)
}

fn read_value_pattern(element: &UIElement) -> Option<String> {
    let pattern = element.get_pattern::<UIValuePattern>().ok()?;
    pattern.get_value().ok().and_then(super::non_empty_text)
}
