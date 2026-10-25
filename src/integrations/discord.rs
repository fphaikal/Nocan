use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use windows::Win32::System::Threading::GetCurrentProcessId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiscordConfig {
    pub api_url: String,
    pub api_key: String,
    pub discord_enabled: bool,
    pub discord_client_id: String,
    pub sync_interval_seconds: u64,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3000/api/activity/now-playing".to_string(),
            api_key: "".to_string(),
            discord_enabled: true,
            discord_client_id: "463151177836658699".to_string(),
            sync_interval_seconds: 3,
        }
    }
}

pub fn load_or_create_config() -> DiscordConfig {
    let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:\\".to_string());
    let mut config_path = PathBuf::from(local_appdata);
    config_path.push("Nocan");
    let _ = std::fs::create_dir_all(&config_path);
    config_path.push("fph_config.json");

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = serde_json::from_str::<DiscordConfig>(&content) {
                return cfg;
            }
        }
    }

    let default_cfg = DiscordConfig::default();
    if let Ok(serialized) = serde_json::to_string_pretty(&default_cfg) {
        let _ = std::fs::write(&config_path, serialized);
    }
    default_cfg
}

pub struct DiscordRpcService {
    config: DiscordConfig,
    running: Arc<AtomicBool>,
}

impl DiscordRpcService {
    pub fn new() -> Self {
        Self {
            config: load_or_create_config(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self, media_service: crate::integrations::media::MediaService) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        if !self.config.discord_enabled {
            return;
        }

        let client_id = self.config.discord_client_id.clone();
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            let _ = std::fs::write("C:\\Users\\abufa\\AppData\\Local\\Nocan\\nocan_discord.log", "Discord service thread started\n");
            let mut ipc = DiscordIpc::new(client_id);
            let mut last_track = String::new();

            while running.load(Ordering::SeqCst) {
                let track = media_service.get_info();
                let current_sig = format!("{}:{}:{}", track.title, track.artist, track.is_playing);

                if current_sig != last_track {
                    last_track = current_sig;

                    if track.is_playing && !track.title.is_empty() {
                        let art_url = get_artwork_url(&track.title, &track.artist, &track.source_app);
                        ipc.set_activity(
                            Some(&track.title),
                            Some(&track.artist),
                            if track.album.is_empty() { None } else { Some(&track.album) },
                            Some(&art_url),
                            &track.source_app,
                            track.duration_ms,
                            track.position_ms,
                            true,
                        );
                    } else {
                        ipc.clear_activity();
                    }
                }

                std::thread::sleep(Duration::from_secs(2));
            }
        });
    }
}

struct DiscordIpc {
    pipe: Option<File>,
    client_id: String,
}

impl DiscordIpc {
    fn new(client_id: String) -> Self {
        Self {
            pipe: None,
            client_id,
        }
    }

    fn ensure_connected(&mut self) -> bool {
        if self.pipe.is_some() {
            return true;
        }

        for i in 0..10 {
            let pipe_name = format!(r"\\.\pipe\discord-ipc-{}", i);
            if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&pipe_name) {
                let handshake = serde_json::json!({
                    "v": 1,
                    "client_id": self.client_id
                });
                let payload = handshake.to_string();
                let frame = encode_frame(0, &payload);
                if file.write_all(&frame).is_ok() {
                    let mut resp_hdr = [0u8; 8];
                    if file.read_exact(&mut resp_hdr).is_ok() {
                        let len = u32::from_le_bytes([resp_hdr[4], resp_hdr[5], resp_hdr[6], resp_hdr[7]]) as usize;
                        let mut resp_body = vec![0u8; len];
                        let _ = file.read_exact(&mut resp_body);
                        self.pipe = Some(file);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn set_activity(
        &mut self,
        details: Option<&str>,
        state: Option<&str>,
        album: Option<&str>,
        artwork_url: Option<&str>,
        source_app: &str,
        duration_ms: u64,
        position_ms: u64,
        is_playing: bool,
    ) {
        if !self.ensure_connected() {
            return;
        }

        let pid = unsafe { GetCurrentProcessId() };
        static NONCE: AtomicU32 = AtomicU32::new(1);
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);

        let activity = if is_playing && details.is_some() {
            let now_secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let (small_icon, small_name) = get_source_icon(source_app);

            let mut timestamps = serde_json::Map::new();
            let start_time = now_secs.saturating_sub(position_ms / 1000);
            timestamps.insert("start".to_string(), serde_json::json!(start_time));

            if duration_ms > 0 && duration_ms > position_ms {
                let end_time = start_time + (duration_ms / 1000);
                timestamps.insert("end".to_string(), serde_json::json!(end_time));
            }

            Some(serde_json::json!({
                "type": 2, // 2 = LISTENING TO
                "details": details.unwrap_or(""),
                "state": state.unwrap_or(""),
                "assets": {
                    "large_image": artwork_url.unwrap_or(small_icon),
                    "large_text": album.unwrap_or("Track Artwork"),
                    "small_image": small_icon,
                    "small_text": small_name
                },
                "timestamps": timestamps,
                "buttons": [
                    { "label": "Portfolio", "url": "https://fph.my.id" }
                ]
            }))
        } else {
            None
        };

        let cmd = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": pid,
                "activity": activity
            },
            "nonce": nonce.to_string()
        });

        let payload = cmd.to_string();
        let frame = encode_frame(1, &payload);

        if let Some(pipe) = &mut self.pipe {
            if pipe.write_all(&frame).is_err() {
                self.pipe = None;
            } else {
                let mut resp_hdr = [0u8; 8];
                let _ = pipe.read_exact(&mut resp_hdr);
            }
        }
    }

    fn clear_activity(&mut self) {
        if !self.ensure_connected() {
            return;
        }

        let pid = unsafe { GetCurrentProcessId() };
        static NONCE: AtomicU32 = AtomicU32::new(1);
        let nonce = NONCE.fetch_add(1, Ordering::Relaxed);

        let cmd = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": pid,
                "activity": serde_json::Value::Null
            },
            "nonce": nonce.to_string()
        });

        let payload = cmd.to_string();
        let frame = encode_frame(1, &payload);

        if let Some(pipe) = &mut self.pipe {
            let _ = pipe.write_all(&frame);
        }
    }
}

fn encode_frame(opcode: u32, payload: &str) -> Vec<u8> {
    let bytes = payload.as_bytes();
    let len = bytes.len() as u32;
    let mut frame = Vec::with_capacity(8 + bytes.len());
    frame.extend_from_slice(&opcode.to_le_bytes());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(bytes);
    frame
}

static ARTWORK_CACHE: Mutex<Option<(String, String)>> = Mutex::new(None);

fn get_artwork_url(title: &str, artist: &str, source_app: &str) -> String {
    let cache_key = format!("{}:{}", title, artist);
    if let Ok(guard) = ARTWORK_CACHE.lock() {
        if let Some((ref k, ref url)) = *guard {
            if k == &cache_key {
                return url.clone();
            }
        }
    }

    let clean_artist = artist
        .replace("- Topic", "")
        .replace("Official Music Video", "")
        .replace("Official Audio", "")
        .replace("Lyric Video", "")
        .trim()
        .to_string();

    let clean_title = title
        .split('(').next().unwrap_or(title)
        .split('[').next().unwrap_or(title)
        .replace("Official Music Video", "")
        .replace("Official Audio", "")
        .replace("Lyric Video", "")
        .trim()
        .to_string();

    let query = format!("{} {}", clean_title, clean_artist);
    let mut encoded = String::new();
    for b in query.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else if b == b' ' {
            encoded.push('+');
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }

    let search_url = format!("https://itunes.apple.com/search?term={}&limit=1&entity=song", encoded);
    let mut found_url: Option<String> = None;

    if let Ok(resp) = ureq::get(&search_url).timeout(Duration::from_secs(3)).call() {
        if let Ok(json) = resp.into_json::<serde_json::Value>() {
            if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
                if let Some(first) = results.first() {
                    if let Some(art) = first.get("artworkUrl100").and_then(|a| a.as_str()) {
                        found_url = Some(art.replace("100x100bb", "512x512bb"));
                    }
                }
            }
        }
    }

    let final_url = found_url.unwrap_or_else(|| {
        let src_lower = source_app.to_lowercase();
        if src_lower.contains("youtube") || src_lower.contains("chrome") || src_lower.contains("msedge") {
            "https://cdn-icons-png.flaticon.com/512/1384/1384060.png".to_string()
        } else if src_lower.contains("spotify") {
            "https://cdn-icons-png.flaticon.com/512/2111/2111624.png".to_string()
        } else if src_lower.contains("apple") {
            "https://cdn-icons-png.flaticon.com/512/831/831276.png".to_string()
        } else {
            "https://cdn-icons-png.flaticon.com/512/3844/3844724.png".to_string()
        }
    });

    if let Ok(mut guard) = ARTWORK_CACHE.lock() {
        *guard = Some((cache_key, final_url.clone()));
    }

    final_url
}

fn get_source_icon(source_app: &str) -> (&'static str, &'static str) {
    let s = source_app.to_lowercase();
    if s.contains("spotify") {
        ("https://cdn-icons-png.flaticon.com/512/2111/2111624.png", "Spotify")
    } else if s.contains("youtube") || s.contains("chrome") || s.contains("msedge") || s.contains("firefox") {
        ("https://cdn-icons-png.flaticon.com/512/1384/1384060.png", "YouTube / Browser")
    } else if s.contains("apple") {
        ("https://cdn-icons-png.flaticon.com/512/831/831276.png", "Apple Music")
    } else {
        ("https://cdn-icons-png.flaticon.com/512/3844/3844724.png", "Media Player")
    }
}
