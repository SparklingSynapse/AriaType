pub trait TextInjector: Send + Sync {
    /// Insert `text` at the current cursor position.
    ///
    /// `write_clipboard`: called only when the AX layer fails and a
    ///   clipboard-based paste is needed. The caller supplies this closure so
    ///   that the injector stays free of any Tauri / platform-clipboard dependency.
    fn insert(&self, text: &str, write_clipboard: &dyn Fn());
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn create_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosInjector);
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsInjector);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    compile_error!("text_injector: unsupported platform");
}
