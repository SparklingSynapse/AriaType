use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};
use tokio::sync::oneshot;

pub use tokio::sync::oneshot::Receiver as OneShotReceiver;
pub use tokio::time::timeout as async_timeout;

pub struct TempFile {
    path: PathBuf,
    _temp_dir: TempDir,
}

impl TempFile {
    pub fn new(data: &[u8], extension: &str) -> Self {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let file_name = format!("test_{}.{}", uuid::Uuid::new_v4(), extension);
        let file_path = temp_dir.path().join(&file_name);
        std::fs::write(&file_path, data).expect("Failed to write temp file");
        Self {
            path: file_path,
            _temp_dir: temp_dir,
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

pub struct TempDirGuard {
    dir: TempDir,
}

impl TempDirGuard {
    pub fn new() -> Self {
        Self {
            dir: tempdir().expect("Failed to create temp directory"),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn create_file(&self, name: &str, data: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, data).expect("Failed to write temp file");
        path
    }
}

impl Default for TempDirGuard {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_channel<T>() -> (oneshot::Sender<T>, Receiver<T>) {
    let (tx, rx) = oneshot::channel();
    (tx, Receiver(rx))
}

pub struct Receiver<T>(oneshot::Receiver<T>);

impl<T> Receiver<T> {
    pub async fn recv(self) -> Result<T, oneshot::error::RecvError> {
        self.0.await
    }
}

#[macro_export]
macro_rules! assert_matches {
    ($expression:expr, $pattern:pat $(if $guard:expr)?) => {
        match $expression {
            $pattern $(if $guard)? => {}
            ref e => panic!("assertion failed: `{:?}` does not match `{}`", e, stringify!($pattern $(if $guard)?)),
        }
    };
}

pub fn run_with_timeout<F>(future: F, duration: std::time::Duration) -> F::Output
where
    F: std::future::Future,
{
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(tokio::time::timeout(duration, future))
        .expect("timed out")
}

pub mod audio {
    use super::*;

    pub fn create_silence_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let samples: Vec<i16> = vec![0i16; num_samples];
        encode_wav(sample_rate, channels, &samples)
    }

    pub fn encode_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            for &sample in samples {
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }
        cursor.into_inner()
    }

    pub fn create_temp_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> TempFile {
        let data = create_silence_wav(sample_rate, channels, duration_secs);
        TempFile::new(&data, "wav")
    }
}

pub mod time {
    use std::future::Future;
    use std::time::{Duration, Instant};

    pub struct Elapsed {
        start: Instant,
    }

    impl Elapsed {
        pub fn start() -> Self {
            Self {
                start: Instant::now(),
            }
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }
    }

    pub async fn measure<F, T>(future: F) -> (T, Duration)
    where
        F: Future<Output = T>,
    {
        let start = Instant::now();
        let result = future.await;
        (result, start.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_temp_file() {
        let temp = TempFile::new(b"test data", "txt");
        assert!(temp.path().exists());
        assert_eq!(std::fs::read(temp.path()).unwrap(), b"test data");
    }

    #[test]
    fn test_temp_dir_guard() {
        let dir = TempDirGuard::new();
        let path = dir.create_file("test.txt", b"content");
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"content");
    }

    #[test]
    fn test_create_channel() {
        let (tx, rx) = create_channel::<i32>();
        tx.send(42).unwrap();
        let result = tokio::runtime::Runtime::new().unwrap().block_on(rx.recv());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_audio_silence_wav() {
        let wav = audio::create_silence_wav(16000, 1, 1.0);
        assert!(!wav.is_empty());
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_audio_create_temp_wav() {
        let temp = audio::create_temp_wav(16000, 1, 0.5);
        assert!(temp.path().exists());
    }

    #[tokio::test]
    async fn test_time_measure() {
        let (result, elapsed) = time::measure(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        })
        .await;
        assert_eq!(result, 42);
        assert!(elapsed >= Duration::from_millis(10));
    }
}
