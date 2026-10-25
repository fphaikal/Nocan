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

            let _ = std::fs::write("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_media.log", "Media service thread started\n");
            while running.load(Ordering::SeqCst) {
                match poll_current_media(&mut cached_key, &mut cached_thumb, &mut thumb_retry_count) {
                    Ok(info) => {
                        if let Ok(mut lock) = current_track.lock() {
                            *lock = info;
                        }
                    }
                    Err(e) => {
                        let _ = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_media.log")
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
    let session = manager.GetCurrentSession()?;

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

    let key = format!("{}|{}", title, artist);
    if !title.is_empty() {
        if key != *cached_key {
            *cached_key = key;
            *thumb_retry_count = 0;
            *cached_thumb = decode_thumbnail_62x62(&media_props).ok().map(Arc::new);
        } else if cached_thumb.is_none() {
            *thumb_retry_count += 1;
            // Persistent retry cadence:
            // - Ticks 1..12 (first 3s): retry every 250ms tick
            // - Ticks 13..40 (3s to 10s): retry every 4 ticks (1s)
            // - Ticks > 40: retry every 20 ticks (5s)
            let should_retry = if *thumb_retry_count <= 12 {
                true
            } else if *thumb_retry_count <= 40 {
                *thumb_retry_count % 4 == 0
            } else {
                *thumb_retry_count % 20 == 0
            };
            if should_retry {
                if let Ok(bytes) = decode_thumbnail_62x62(&media_props) {
                    *cached_thumb = Some(Arc::new(bytes));
                }
            }
        }
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
