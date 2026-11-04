use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapBounds, BitmapDecoder, BitmapInterpolationMode, BitmapPixelFormat,
    BitmapTransform, ColorManagementMode, ExifOrientationMode,
};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[derive(Clone, Debug, Default)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source_app: String,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub artwork_url: Option<String>,
    pub thumbnail_bgra: Option<Arc<Vec<u8>>>, // 62x62 BGRA8 Premultiplied
}

#[derive(Clone)]
pub struct MediaService {
    current_track: Arc<Mutex<MediaTrackInfo>>,
    running: Arc<AtomicBool>,
}

impl MediaService {
    pub fn new() -> Self {
        Self {
            current_track: Arc::new(Mutex::new(MediaTrackInfo::default())),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let current_track = Arc::clone(&self.current_track);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            // Initialize COM for WinRT
            unsafe {
                let _ = windows::Win32::System::Com::CoInitializeEx(
                    None,
                    windows::Win32::System::Com::COINIT_MULTITHREADED,
                );
            }

            let mut cached_key = String::new();
            let mut cached_thumb: Option<Arc<Vec<u8>>> = None;
            let mut thumb_retry_count: u32 = 0;

            let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\Users\\abufa\\AppData\\Local".to_string());
            let log_dir = std::path::PathBuf::from(base).join("Nocan");
            let _ = std::fs::create_dir_all(&log_dir);
            let media_log = log_dir.join("nocan_media.log");

            let _ = std::fs::write(&media_log, "Media service thread started\n");
            let mut last_error_log: Option<std::time::Instant> = None;
            while running.load(Ordering::SeqCst) {
                match poll_current_media(&mut cached_key, &mut cached_thumb, &mut thumb_retry_count) {
                    Ok(info) => {
                        last_error_log = None;
                        if let Ok(mut lock) = current_track.lock() {
                            *lock = info;
                        }
                    }
                    Err(e) => {
                        if last_error_log.is_some_and(|last| last.elapsed() < Duration::from_secs(60)) {
                            std::thread::sleep(Duration::from_millis(250));
                            continue;
                        }
                        last_error_log = Some(std::time::Instant::now());
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&media_log)
                            .and_then(|mut f| {
                                use std::io::Write;
                                writeln!(f, "poll_current_media err: {:?}", e)
                            });
                    }
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }

    pub fn get_info(&self) -> MediaTrackInfo {
        self.current_track.lock().map(|t| t.clone()).unwrap_or_default()
    }

    pub fn toggle_play_pause(&self) {
        std::thread::spawn(|| {
            if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = manager.get() {
                    if let Ok(session) = mgr.GetCurrentSession() {
                        let _ = session.TryTogglePlayPauseAsync();
                    }
                }
            }
        });
    }

    pub fn skip_next(&self) {
        std::thread::spawn(|| {
            if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = manager.get() {
                    if let Ok(session) = mgr.GetCurrentSession() {
                        let _ = session.TrySkipNextAsync();
                    }
                }
            }
        });
    }

    pub fn skip_prev(&self) {
        std::thread::spawn(|| {
            if let Ok(manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                if let Ok(mgr) = manager.get() {
                    if let Ok(session) = mgr.GetCurrentSession() {
                        let _ = session.TrySkipPreviousAsync();
                    }
                }
            }
        });
    }
}

fn poll_current_media(
    cached_key: &mut String,
    cached_thumb: &mut Option<Arc<Vec<u8>>>,
    thumb_retry_count: &mut u32,
) -> windows::core::Result<MediaTrackInfo> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()?;
    let session = match manager.GetCurrentSession() {
        Ok(s) => {
            let is_cur_playing = s
                .GetPlaybackInfo()
                .and_then(|info| info.PlaybackStatus())
                .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                .unwrap_or(false);

            if is_cur_playing {
                s
            } else if let Ok(sessions) = manager.GetSessions() {
                let mut playing_session = None;
                for other in sessions {
                    if let Ok(info) = other.GetPlaybackInfo() {
                        if let Ok(status) = info.PlaybackStatus() {
                            if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                                playing_session = Some(other);
                                break;
                            }
                        }
                    }
                }
                playing_session.unwrap_or(s)
            } else {
                s
            }
        }
        Err(_) => {
            let sessions = manager.GetSessions()?;
            let mut found = None;
            for s in &sessions {
                if let Ok(info) = s.GetPlaybackInfo() {
                    if let Ok(status) = info.PlaybackStatus() {
                        if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                            found = Some(s);
                            break;
                        }
                    }
                }
            }
            if let Some(s) = found {
                s
            } else if let Ok(first) = sessions.GetAt(0) {
                first
            } else {
                return Err(windows::core::Error::empty());
            }
        }
    };

    let source_app = session
        .SourceAppUserModelId()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "Media".to_string());

    let playback_info = session.GetPlaybackInfo()?;
    let is_playing = playback_info.PlaybackStatus()?
        == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let media_props = session.TryGetMediaPropertiesAsync()?.get()?;
    let title = media_props.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = media_props.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = media_props
        .AlbumTitle()
        .map(|s| s.to_string())
        .unwrap_or_default();

    let key = format!("{}|{}|{}|{}", source_app, title, artist, album);
    if !title.is_empty() {
        refresh_thumbnail(&key, cached_key, cached_thumb, thumb_retry_count,
            || decode_thumbnail_62x62(&media_props).ok());
    } else {
        cached_key.clear();
        *cached_thumb = None;
        *thumb_retry_count = 0;
    }

    let mut position_ms = 0;
    let mut duration_ms = 0;
    if let Ok(timeline) = session.GetTimelineProperties() {
        if let Ok(pos) = timeline.Position() {
            position_ms = (pos.Duration / 10_000) as u64;
        }
        if is_playing {
            if let Ok(lu) = timeline.LastUpdatedTime() {
                // Browsers report a stale Position; extrapolate from the time it was last updated.
                let now_ft = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros() as i64).unwrap_or(0) + 11_644_473_600_000_000) * 10;
                let elapsed_ms = ((now_ft - lu.UniversalTime) / 10_000).clamp(0, 6 * 3600 * 1000);
                if lu.UniversalTime > 0 {
                    position_ms += elapsed_ms as u64;
                }
            }
        }
        if let Ok(end) = timeline.EndTime() {
            duration_ms = (end.Duration / 10_000) as u64;
        }
    }

    if duration_ms > 0 && position_ms > duration_ms {
        position_ms = duration_ms;
    }

    Ok(MediaTrackInfo {
        title,
        artist,
        album,
        source_app,
        is_playing,
        position_ms,
        duration_ms,
        artwork_url: None,
        thumbnail_bgra: cached_thumb.clone(),
    })
}

fn refresh_thumbnail<F>(key: &str, cached_key: &mut String,
    cached_thumb: &mut Option<Arc<Vec<u8>>>, ticks: &mut u32, decode: F)
where F: FnOnce() -> Option<Vec<u8>> {
    let changed = key != cached_key;
    if changed {
        *cached_key = key.to_owned();
        *cached_thumb = None;
        *ticks = 0;
    } else {
        *ticks = ticks.saturating_add(1);
    }

    // Some players initially publish an app icon and replace it with artwork later.
    let refresh = changed || if cached_thumb.is_none() && *ticks <= 12 {
        true
    } else if *ticks <= 80 {
        *ticks % 4 == 0
    } else {
        *ticks % 20 == 0
    };
    if refresh {
        if let Some(bytes) = decode() {
            if cached_thumb.as_ref().map_or(true, |current| current.as_ref() != &bytes) {
                *cached_thumb = Some(Arc::new(bytes));
            }
        }
    }
}

#[cfg(test)]
mod thumbnail_tests {
    use super::*;

    #[test]
    fn replaces_late_artwork_and_keeps_last_image_on_failure() {
        let (mut key, mut image, mut ticks) = (String::new(), None, 0);
        refresh_thumbnail("song", &mut key, &mut image, &mut ticks, || Some(vec![1]));
        let original = image.as_ref().unwrap().clone();
        for _ in 0..3 {
            refresh_thumbnail("song", &mut key, &mut image, &mut ticks, || panic!("too early"));
        }
        refresh_thumbnail("song", &mut key, &mut image, &mut ticks, || None);
        assert!(Arc::ptr_eq(image.as_ref().unwrap(), &original));
        for _ in 0..3 {
            refresh_thumbnail("song", &mut key, &mut image, &mut ticks, || panic!("too early"));
        }
        refresh_thumbnail("song", &mut key, &mut image, &mut ticks, || Some(vec![2]));
        assert_eq!(image.as_ref().unwrap().as_ref(), &vec![2]);
        refresh_thumbnail("next", &mut key, &mut image, &mut ticks, || None);
        assert!(image.is_none());
        refresh_thumbnail("next", &mut key, &mut image, &mut ticks, || Some(vec![3]));
        assert_eq!(image.as_ref().unwrap().as_ref(), &vec![3]);
    }
}

fn decode_thumbnail_62x62(
    media_props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
) -> windows::core::Result<Vec<u8>> {
    let thumb_ref = media_props.Thumbnail()?;
    let stream = thumb_ref.OpenReadAsync()?.get()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;
    let orig_w = decoder.PixelWidth()?;
    let orig_h = decoder.PixelHeight()?;
    if orig_w == 0 || orig_h == 0 {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(-1)));
    }

    let transform = BitmapTransform::new()?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Linear)?;

    // Scale so the smaller dimension becomes 62, then center-crop 62x62
    let target = 62u32;
    let (scaled_w, scaled_h) = if orig_w >= orig_h {
        let sw = ((orig_w as u64 * target as u64) / orig_h as u64).max(target as u64) as u32;
        (sw, target)
    } else {
        let sh = ((orig_h as u64 * target as u64) / orig_w as u64).max(target as u64) as u32;
        (target, sh)
    };
    transform.SetScaledWidth(scaled_w)?;
    transform.SetScaledHeight(scaled_h)?;
    transform.SetBounds(BitmapBounds {
        X: (scaled_w.saturating_sub(target)) / 2,
        Y: (scaled_h.saturating_sub(target)) / 2,
        Width: target,
        Height: target,
    })?;

    let provider = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transform,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?
        .get()?;
    let bytes = provider.DetachPixelData()?;
    Ok(bytes.to_vec())
}
