use std::sync::Arc;

use fast_image_resize::images::Image as ResizeImage;
use fast_image_resize::pixels::PixelType;
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use tracing::{debug, info, warn};
use uni_ocr::{OcrEngine, OcrOptions, OcrProvider};

use crate::runtime_context::window::{
    OcrConfidenceSummary, WindowContextBundle, WindowContextSource,
};
use crate::utils::AppPaths;

const MAX_IMAGE_WIDTH: u32 = 2048;
const RESIZE_IMAGE_BEFORE_OCR: bool = false;
const RESIZE_SKIP_MARGIN: u32 = 128;
const MIN_FOCUSED_WINDOW_WIDTH: u32 = 240;
const MIN_FOCUSED_WINDOW_HEIGHT: u32 = 120;
const DEBUG_SAVE_OCR_SCREENSHOTS_ENV: &str = "ARIATYPE_DEBUG_SAVE_OCR_SCREENSHOTS";

struct CapturedWindowImage {
    image: image::DynamicImage,
    source: WindowContextSource,
    window_title: Option<String>,
}

fn resize_image(img: image::DynamicImage, max_width: u32) -> image::DynamicImage {
    if !RESIZE_IMAGE_BEFORE_OCR {
        return img;
    }

    let width = img.width();
    if width <= max_width.saturating_add(RESIZE_SKIP_MARGIN) {
        return img;
    }
    let ratio = max_width as f32 / width as f32;
    let new_height = (img.height() as f32 * ratio).round() as u32;

    let source_height = img.height();
    let mut source_buffer = img.into_rgba8().into_raw();

    let mut destination = ResizeImage::new(max_width, new_height, PixelType::U8x4);
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Hamming));
    let mut resizer = Resizer::new();
    let resize_result = {
        let source = match ResizeImage::from_slice_u8(
            width,
            source_height,
            &mut source_buffer,
            PixelType::U8x4,
        ) {
            Ok(source) => source,
            Err(e) => {
                warn!(error = %e, "window_context_fast_resize_source_failed");
                return image::DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(width, source_height, source_buffer)
                        .unwrap_or_else(|| image::ImageBuffer::new(width, source_height)),
                )
                .resize_exact(
                    max_width,
                    new_height,
                    image::imageops::FilterType::Triangle,
                );
            }
        };
        resizer.resize(&source, &mut destination, &options)
    };
    if let Err(e) = resize_result {
        warn!(error = %e, "window_context_fast_resize_failed");
        return image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(width, source_height, source_buffer)
                .unwrap_or_else(|| image::ImageBuffer::new(width, source_height)),
        )
        .resize_exact(max_width, new_height, image::imageops::FilterType::Triangle);
    }

    image::RgbaImage::from_raw(max_width, new_height, destination.into_vec())
        .map(image::DynamicImage::ImageRgba8)
        .unwrap_or_else(|| {
            warn!("window_context_fast_resize_destination_failed");
            image::DynamicImage::ImageRgba8(image::ImageBuffer::new(max_width, new_height))
        })
}

fn ocr_screenshot_path() -> std::path::PathBuf {
    let filename = format!("{}.png", chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S-%3f"));
    AppPaths::log_dir().join(filename)
}

fn debug_save_ocr_screenshots_enabled() -> bool {
    std::env::var(DEBUG_SAVE_OCR_SCREENSHOTS_ENV)
        .map(|value| is_truthy_env_value(&value))
        .unwrap_or(false)
}

fn is_truthy_env_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn save_ocr_screenshot_to_path(image: &image::DynamicImage, path: &std::path::Path) -> bool {
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            warn!(error = %e, path = %parent.display(), "window_context_screenshot_dir_create_failed");
            return false;
        }
    }

    match image.save(path) {
        Ok(()) => true,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "window_context_screenshot_save_failed");
            false
        }
    }
}

fn schedule_ocr_screenshot_save(image: Arc<image::DynamicImage>) -> Option<std::path::PathBuf> {
    if !debug_save_ocr_screenshots_enabled() {
        debug!("window_context_screenshot_save_disabled");
        return None;
    }

    let path = ocr_screenshot_path();
    let path_for_task = path.clone();

    std::mem::drop(tokio::task::spawn_blocking(move || {
        let started = std::time::Instant::now();
        if save_ocr_screenshot_to_path(image.as_ref(), &path_for_task) {
            info!(
                save_ms = started.elapsed().as_millis(),
                path = %path_for_task.display(),
                "window_context_screenshot_saved"
            );
        }
    }));

    Some(path)
}

pub async fn capture_window_context() -> Option<WindowContextBundle> {
    let total_started = std::time::Instant::now();
    let capture_started = std::time::Instant::now();
    let captured = tokio::task::spawn_blocking(capture_focused_window_image_blocking)
        .await
        .ok()
        .flatten();
    let capture_ms = capture_started.elapsed().as_millis();

    let captured = match captured {
        Some(captured) => captured,
        None => {
            debug!("window_context_capture_no_image");
            return None;
        }
    };

    let source_image_width = captured.image.width();
    let source_image_height = captured.image.height();

    let resize_started = std::time::Instant::now();
    let image = resize_image(captured.image, MAX_IMAGE_WIDTH);
    let resize_ms = resize_started.elapsed().as_millis();

    let grayscale_ms = 0;
    let contrast_ms = 0;
    let sharpen_ms = 0;
    let image = Arc::new(image);

    let image_width = image.width();
    let image_height = image.height();

    let save_started = std::time::Instant::now();
    let screenshot_path = schedule_ocr_screenshot_save(Arc::clone(&image));
    let save_ms = save_started.elapsed().as_millis();
    let screenshot_saved = screenshot_path.is_some();

    info!(
        capture_ms,
        resize_ms,
        grayscale_ms,
        contrast_ms,
        sharpen_ms,
        save_ms,
        total_ms = total_started.elapsed().as_millis(),
        source_width = source_image_width,
        source_height = source_image_height,
        width = image_width,
        height = image_height,
        screenshot_saved,
        "window_context_ocr_input_ready"
    );

    let ocr_init_started = std::time::Instant::now();
    let engine = match OcrEngine::new(OcrProvider::Auto) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "ocr_engine_init_failed");
            return None;
        }
    };
    let ocr_init_ms = ocr_init_started.elapsed().as_millis();

    let engine = engine.with_options(OcrOptions::default());

    let ocr_started = std::time::Instant::now();
    info!("window_context_ocr_started");
    let ocr_result = engine.recognize_image(image.as_ref()).await;
    let ocr_ms = ocr_started.elapsed().as_millis();

    info!(
        capture_ms,
        resize_ms,
        grayscale_ms,
        contrast_ms,
        sharpen_ms,
        save_ms,
        ocr_init_ms,
        ocr_ms,
        total_ms = total_started.elapsed().as_millis(),
        source_width = source_image_width,
        source_height = source_image_height,
        width = image_width,
        height = image_height,
        "window_context_timing"
    );

    match ocr_result {
        Ok((text, detailed, confidence)) => {
            let confidence_summary = summarize_ocr_confidence(confidence, &detailed);
            let Some(bundle) = WindowContextBundle::from_ocr_result_with_confidence(
                text,
                captured.source,
                captured.window_title,
                image_width,
                image_height,
                confidence_summary,
            ) else {
                debug!("window_context_ocr_empty");
                return None;
            };
            info!(
                chars = bundle.filtered_text.len(),
                source = bundle.source.as_str(),
                screenshot_saved,
                ocr_confidence_avg = ?bundle.ocr_confidence.map(|confidence| confidence.average),
                ocr_confidence_max = ?bundle.ocr_confidence.map(|confidence| confidence.max),
                ocr_observations = ?bundle.ocr_confidence.map(|confidence| confidence.observations),
                ocr_provider_raw_confidence = ?bundle.ocr_confidence.and_then(|confidence| confidence.provider_raw),
                raw_ocr_chars = bundle.raw_ocr_text.chars().count(),
                filtered_text_chars = bundle.filtered_text.chars().count(),
                "window_context_captured"
            );
            Some(bundle)
        }
        Err(e) => {
            warn!(error = %e, "window_context_ocr_failed");
            None
        }
    }
}

fn summarize_ocr_confidence(
    provider_raw: Option<f64>,
    detailed_json: &str,
) -> Option<OcrConfidenceSummary> {
    let confidences = extract_detailed_confidences(detailed_json);
    if confidences.is_empty() {
        return provider_raw.and_then(OcrConfidenceSummary::from_single_score);
    }

    let sum: f64 = confidences.iter().sum();
    let max_observed = confidences
        .iter()
        .fold(0.0_f64, |current, value| current.max(*value));
    let max = if max_observed <= 1.0 { 1.0 } else { 100.0 };

    OcrConfidenceSummary::new(
        sum / confidences.len() as f64,
        max,
        confidences.len(),
        provider_raw,
    )
}

fn extract_detailed_confidences(detailed_json: &str) -> Vec<f64> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(detailed_json) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| item.get("conf").or_else(|| item.get("confidence")))
        .filter_map(parse_json_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .collect()
}

fn parse_json_f64(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn capture_focused_window_image_blocking() -> Option<CapturedWindowImage> {
    let total_started = std::time::Instant::now();
    let list_started = std::time::Instant::now();
    let windows = match xcap::Window::all() {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "window_list_failed");
            return capture_primary_monitor_blocking();
        }
    };
    let list_ms = list_started.elapsed().as_millis();
    let window_count = windows.len();

    // xcap::Window::all() returns windows in z-order (topmost first).
    // is_focused() checks at app level (PID), so the first focused+visible
    // window is the frontmost window of the active app.
    let select_started = std::time::Instant::now();
    let focused = windows
        .iter()
        .filter(|w| !w.is_minimized().unwrap_or(true) && w.is_focused().unwrap_or(false))
        .find(|w| {
            let title = w.title().unwrap_or_default();
            let width = w.width().unwrap_or(0);
            let height = w.height().unwrap_or(0);
            let usable = is_usable_focused_window(&title, width, height);
            if !usable {
                info!(
                    title = %title,
                    width,
                    height,
                    "focused_window_ignored_unusable"
                );
            }
            usable
        });
    let select_ms = select_started.elapsed().as_millis();

    let window = match focused {
        Some(w) => w,
        None => {
            debug!("no_focused_window_found, falling back to monitor");
            return capture_primary_monitor_blocking();
        }
    };

    let title = window.title().unwrap_or_default();
    let width = window.width().unwrap_or(0);
    let height = window.height().unwrap_or(0);
    let capture_started = std::time::Instant::now();
    match window.capture_image() {
        Ok(rgba) => {
            let capture_image_ms = capture_started.elapsed().as_millis();
            info!(
                title = %title,
                width,
                height,
                window_count,
                window_list_ms = list_ms,
                select_ms,
                capture_image_ms,
                total_ms = total_started.elapsed().as_millis(),
                "focused_window_captured"
            );
            Some(CapturedWindowImage {
                image: image::DynamicImage::ImageRgba8(rgba),
                source: WindowContextSource::FocusedWindow,
                window_title: Some(title),
            })
        }
        Err(e) => {
            warn!(
                error = %e,
                title = %title,
                window_count,
                window_list_ms = list_ms,
                select_ms,
                capture_image_ms = capture_started.elapsed().as_millis(),
                total_ms = total_started.elapsed().as_millis(),
                "window_capture_failed"
            );
            capture_primary_monitor_blocking()
        }
    }
}

fn is_usable_focused_window(title: &str, width: u32, height: u32) -> bool {
    !title.trim().is_empty()
        && width >= MIN_FOCUSED_WINDOW_WIDTH
        && height >= MIN_FOCUSED_WINDOW_HEIGHT
}

fn capture_primary_monitor_blocking() -> Option<CapturedWindowImage> {
    let monitors = match xcap::Monitor::all() {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "monitor_list_failed");
            return None;
        }
    };

    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false));
    let monitor = primary.or_else(|| xcap::Monitor::from_point(0, 0).ok())?;

    match monitor.capture_image() {
        Ok(rgba) => {
            info!("monitor_fallback_captured");
            Some(CapturedWindowImage {
                image: image::DynamicImage::ImageRgba8(rgba),
                source: WindowContextSource::PrimaryMonitor,
                window_title: None,
            })
        }
        Err(e) => {
            warn!(error = %e, "monitor_capture_failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgba};

    #[test]
    fn empty_string_returns_none_after_trim() {
        let text = "   ";
        let trimmed = text.trim();
        assert!(trimmed.is_empty());
    }

    #[test]
    fn usable_focused_window_requires_title_and_reasonable_size() {
        assert!(is_usable_focused_window("Editor", 1024, 768));
        assert!(!is_usable_focused_window("", 1024, 768));
        assert!(!is_usable_focused_window("Menu bar", 1920, 32));
        assert!(!is_usable_focused_window("Narrow", 120, 600));
    }

    #[test]
    fn ocr_screenshot_path_uses_log_dir_and_png_extension() {
        let path = ocr_screenshot_path();
        assert_eq!(path.parent(), Some(AppPaths::log_dir().as_path()));
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("png"));
        assert!(path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.len() >= "2026-05-11-12-57-35-000".len()));
    }

    #[test]
    fn debug_screenshot_env_accepts_only_explicit_truthy_values() {
        assert!(is_truthy_env_value("1"));
        assert!(is_truthy_env_value("true"));
        assert!(is_truthy_env_value("YES"));
        assert!(is_truthy_env_value(" on "));
        assert!(!is_truthy_env_value(""));
        assert!(!is_truthy_env_value("0"));
        assert!(!is_truthy_env_value("false"));
    }

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(width, height, Rgba([128, 128, 128, 255]));
        DynamicImage::ImageRgba8(buffer)
    }

    #[test]
    fn resize_image_keeps_small_images_unchanged() {
        let img = create_test_image(800, 600);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), 800);
        assert_eq!(resized.height(), 600);
    }

    #[test]
    fn resize_image_keeps_large_images_unchanged_when_resize_is_disabled() {
        let img = create_test_image(3072, 1882);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), 3072);
        assert_eq!(resized.height(), 1882);
    }

    #[test]
    fn resize_image_skips_near_max_width_images() {
        let img = create_test_image(MAX_IMAGE_WIDTH + RESIZE_SKIP_MARGIN, 972);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), MAX_IMAGE_WIDTH + RESIZE_SKIP_MARGIN);
        assert_eq!(resized.height(), 972);
    }

    #[test]
    fn resize_image_keeps_max_width_images_unchanged() {
        let img = create_test_image(MAX_IMAGE_WIDTH, 900);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), MAX_IMAGE_WIDTH);
        assert_eq!(resized.height(), 900);
    }

    #[test]
    fn resize_image_preserves_large_image_dimensions_when_resize_is_disabled() {
        let img = create_test_image(4096, 2304);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), 4096);
        assert_eq!(resized.height(), 2304);
    }

    #[test]
    fn confidence_summary_averages_apple_observation_scores() {
        let detailed = r#"
        [
            {"conf": "0.80", "text": "AriaType"},
            {"conf": "0.60", "text": "README"}
        ]
        "#;

        let summary = summarize_ocr_confidence(Some(1.40), detailed).unwrap();

        assert_eq!(summary.average, 0.70);
        assert_eq!(summary.max, 1.0);
        assert_eq!(summary.observations, 2);
        assert_eq!(summary.provider_raw, Some(1.40));
    }

    #[test]
    fn confidence_summary_handles_percent_scale_scores() {
        let detailed = r#"
        [
            {"confidence": "80.00", "text": "AriaType"},
            {"confidence": "60.00", "text": "README"}
        ]
        "#;

        let summary = summarize_ocr_confidence(Some(70.0), detailed).unwrap();

        assert_eq!(summary.average, 70.0);
        assert_eq!(summary.max, 100.0);
        assert_eq!(summary.observations, 2);
        assert_eq!(summary.provider_raw, Some(70.0));
    }

    #[test]
    fn confidence_summary_falls_back_to_provider_score() {
        let summary = summarize_ocr_confidence(Some(0.83), "not json").unwrap();

        assert_eq!(summary.average, 0.83);
        assert_eq!(summary.max, 1.0);
        assert_eq!(summary.observations, 0);
        assert_eq!(summary.provider_raw, Some(0.83));
    }
}
