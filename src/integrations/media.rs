use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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

            let _ = std::fs::write("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_media.log", "Media service thread started\n");
            while running.load(Ordering::SeqCst) {
                match poll_current_media() {
                    Ok(info) => {
                        if let Ok(mut lock) = current_track.lock() {
                            *lock = info;
                        }
                    }
                    Err(_e) => {
                        // Suppress expected "no session" errors
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
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

fn poll_current_media() -> windows::core::Result<MediaTrackInfo> {
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

    let mut position_ms = 0;
    let mut duration_ms = 0;
    if let Ok(timeline) = session.GetTimelineProperties() {
        if let Ok(pos) = timeline.Position() {
            position_ms = (pos.Duration / 10_000) as u64;
        }
        if let Ok(end) = timeline.EndTime() {
            duration_ms = (end.Duration / 10_000) as u64;
        }
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
    })
}
