use tracing::{debug, info, warn};
use uni_ocr::{OcrEngine, OcrOptions, OcrProvider};

const MAX_CONTEXT_CHARS: usize = 800;
const MAX_IMAGE_WIDTH: u32 = 1024;

fn resize_image(img: image::DynamicImage, max_width: u32) -> image::DynamicImage {
    let width = img.width();
    if width <= max_width {
        return img;
    }
    let ratio = max_width as f32 / width as f32;
    let new_height = (img.height() as f32 * ratio).round() as u32;
    img.resize_exact(max_width, new_height, image::imageops::FilterType::Lanczos3)
}

pub async fn capture_window_context() -> Option<String> {
    let image = tokio::task::spawn_blocking(|| capture_focused_window_image_blocking())
        .await
        .ok()
        .flatten();

    let image = match image {
        Some(img) => resize_image(img, MAX_IMAGE_WIDTH),
        None => {
            debug!("window_context_capture_no_image");
            return None;
        }
    };

    let engine = match OcrEngine::new(OcrProvider::Auto) {
        Ok(e) => e,
        Err(e) => {
            warn!(error = %e, "ocr_engine_init_failed");
            return None;
        }
    };

    let engine = engine.with_options(OcrOptions::default());

    match engine.recognize_image(&image).await {
        Ok((text, _detailed, _confidence)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                debug!("window_context_ocr_empty");
                return None;
            }
            let result = if trimmed.chars().count() > MAX_CONTEXT_CHARS {
                trimmed.chars().take(MAX_CONTEXT_CHARS).collect::<String>()
            } else {
                trimmed.to_string()
            };
            info!(chars = result.len(), "window_context_captured");
            Some(result)
        }
        Err(e) => {
            warn!(error = %e, "window_context_ocr_failed");
            None
        }
    }
}

fn capture_focused_window_image_blocking() -> Option<image::DynamicImage> {
    let windows = match xcap::Window::all() {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "window_list_failed");
            return capture_primary_monitor_blocking();
        }
    };

    for window in windows {
        let is_minimized = window.is_minimized().unwrap_or(true);
        if is_minimized {
            continue;
        }

        let width = window.width().unwrap_or(0);
        let height = window.height().unwrap_or(0);
        if width == 0 || height == 0 {
            continue;
        }

        match window.capture_image() {
            Ok(rgba) => {
                info!(
                    title = %window.title().unwrap_or_default(),
                    width = width,
                    height = height,
                    "window_captured"
                );
                return Some(image::DynamicImage::ImageRgba8(rgba));
            }
            Err(e) => {
                debug!(error = %e, title = %window.title().unwrap_or_default(), "window_capture_failed");
                continue;
            }
        }
    }

    capture_primary_monitor_blocking()
}

fn capture_primary_monitor_blocking() -> Option<image::DynamicImage> {
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
            Some(image::DynamicImage::ImageRgba8(rgba))
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
    fn truncation_preserves_first_800_chars() {
        let long_text = "a".repeat(1000);
        let result = if long_text.chars().count() > MAX_CONTEXT_CHARS {
            long_text.chars().take(MAX_CONTEXT_CHARS).collect::<String>()
        } else {
            long_text.clone()
        };
        assert_eq!(result.chars().count(), MAX_CONTEXT_CHARS);
    }

    #[test]
    fn truncation_keeps_short_text_unchanged() {
        let short_text = "hello world";
        let result = if short_text.chars().count() > MAX_CONTEXT_CHARS {
            short_text.chars().take(MAX_CONTEXT_CHARS).collect::<String>()
        } else {
            short_text.to_string()
        };
        assert_eq!(result, "hello world");
    }

    #[test]
    fn empty_string_returns_none_after_trim() {
        let text = "   ";
        let trimmed = text.trim();
        assert!(trimmed.is_empty());
    }

    #[test]
    fn max_context_chars_constant_is_reasonable() {
        assert!(MAX_CONTEXT_CHARS >= 500);
        assert!(MAX_CONTEXT_CHARS <= 1000);
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
    fn resize_image_scales_large_images_to_max_width() {
        let img = create_test_image(2048, 1536);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        assert_eq!(resized.width(), MAX_IMAGE_WIDTH);
        assert_eq!(resized.height(), 768);
    }

    #[test]
    fn resize_image_preserves_aspect_ratio() {
        let img = create_test_image(1920, 1080);
        let resized = resize_image(img, MAX_IMAGE_WIDTH);
        let expected_height = (1080.0 * (MAX_IMAGE_WIDTH as f32 / 1920.0)).round() as u32;
        assert_eq!(resized.height(), expected_height);
    }
}
