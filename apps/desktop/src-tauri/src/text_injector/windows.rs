pub struct WindowsInjector;

impl super::TextInjector for WindowsInjector {
    fn insert(&self, _text: &str, _write_clipboard: &dyn Fn()) {
        // TODO: SendInput / UI Automation
    }
}
