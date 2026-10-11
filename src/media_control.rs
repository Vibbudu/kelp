use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaState {
    pub is_active: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub thumbnail_base64: Option<String>,
    pub source_app: String,
    pub can_skip_next: bool,
    pub can_skip_prev: bool,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            is_active: false,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            is_playing: false,
            thumbnail_base64: None,
            source_app: String::new(),
            can_skip_next: true,
            can_skip_prev: true,
        }
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::MediaState;
    use tracing::{debug, warn};
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        keybd_event, KEYEVENTF_KEYUP, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
        VK_MEDIA_PREV_TRACK,
    };

    fn format_source_app(raw_id: &str) -> String {
        let lower = raw_id.to_lowercase();
        if lower.contains("spotify") {
            "Spotify".to_string()
        } else if lower.contains("chrome") {
            "Google Chrome".to_string()
        } else if lower.contains("msedge") || lower.contains("edge") {
            "Microsoft Edge".to_string()
        } else if lower.contains("brave") {
            "Brave".to_string()
        } else if lower.contains("firefox") {
            "Firefox".to_string()
        } else if lower.contains("applemusic") || lower.contains("apple music") {
            "Apple Music".to_string()
        } else if lower.contains("vlc") {
            "VLC".to_string()
        } else if lower.contains("tidal") {
            "Tidal".to_string()
        } else if lower.contains("music") {
            "Media Player".to_string()
        } else {
            // Strip .exe and path if present
            std::path::Path::new(raw_id)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| raw_id.to_string())
        }
    }

    fn extract_thumbnail_base64(
        session: &GlobalSystemMediaTransportControlsSession,
    ) -> Option<String> {
        let media_props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let thumb_ref = media_props.Thumbnail().ok()?;
        let stream = thumb_ref.OpenReadAsync().ok()?.get().ok()?;
        let size = stream.Size().ok()? as usize;
        if size == 0 || size > 5 * 1024 * 1024 {
            return None;
        }

        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size as u32).ok()?.get().ok()?;
        let mut buffer = vec![0u8; size];
        reader.ReadBytes(&mut buffer).ok()?;

        Some(crate::utilities::to_base64(&buffer))
    }

    pub fn get_current_media_state() -> Option<MediaState> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .ok()?
            .get()
            .ok()?;

        // 1. Try current active session first
        let session = if let Ok(s) = manager.GetCurrentSession() {
            s
        } else {
            // 2. Fallback to iterating sessions
            let sessions = manager.GetSessions().ok()?;
            let count = sessions.Size().ok()?;
            if count == 0 {
                return None;
            }

            // Find first playing session, or first valid session
            let mut found = None;
            for i in 0..count {
                if let Ok(s) = sessions.GetAt(i) {
                    if let Ok(info) = s.GetPlaybackInfo() {
                        if let Ok(status) = info.PlaybackStatus() {
                            if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                                found = Some(s);
                                break;
                            }
                        }
                    }
                    if found.is_none() {
                        found = Some(s);
                    }
                }
            }
            found?
        };

        let media_props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = media_props.Title().unwrap_or_default().to_string();
        let artist = media_props.Artist().unwrap_or_default().to_string();
        let album = media_props.AlbumTitle().unwrap_or_default().to_string();

        if title.is_empty() && artist.is_empty() {
            return None;
        }

        let playback_info = session.GetPlaybackInfo().ok();
        let is_playing = playback_info
            .as_ref()
            .and_then(|p| p.PlaybackStatus().ok())
            .map(|s| s == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
            .unwrap_or(false);

        let controls = playback_info.as_ref().and_then(|p| p.Controls().ok());
        let can_skip_next = controls
            .as_ref()
            .and_then(|c| c.IsNextEnabled().ok())
            .unwrap_or(true);
        let can_skip_prev = controls
            .as_ref()
            .and_then(|c| c.IsPreviousEnabled().ok())
            .unwrap_or(true);

        let raw_app_id = session
            .SourceAppUserModelId()
            .unwrap_or_default()
            .to_string();
        let source_app = format_source_app(&raw_app_id);

        let thumbnail_base64 = extract_thumbnail_base64(&session);

        Some(MediaState {
            is_active: true,
            title,
            artist,
            album,
            is_playing,
            thumbnail_base64,
            source_app,
            can_skip_next,
            can_skip_prev,
        })
    }

    pub fn send_media_command(action: &str) -> Option<MediaState> {
        let manager_res = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .ok()
            .and_then(|op| op.get().ok());

        let mut executed_via_api = false;

        if let Some(manager) = manager_res {
            if let Ok(session) = manager.GetCurrentSession() {
                match action {
                    "play" => {
                        executed_via_api = session.TryPlayAsync().ok().and_then(|op| op.get().ok()).unwrap_or(false);
                    }
                    "pause" => {
                        executed_via_api = session.TryPauseAsync().ok().and_then(|op| op.get().ok()).unwrap_or(false);
                    }
                    "toggle" => {
                        executed_via_api = session.TryTogglePlayPauseAsync().ok().and_then(|op| op.get().ok()).unwrap_or(false);
                    }
                    "next" => {
                        executed_via_api = session.TrySkipNextAsync().ok().and_then(|op| op.get().ok()).unwrap_or(false);
                    }
                    "prev" => {
                        executed_via_api = session.TrySkipPreviousAsync().ok().and_then(|op| op.get().ok()).unwrap_or(false);
                    }
                    _ => warn!("Unknown media command: {}", action),
                }
            }
        }

        // Fallback to Win32 Virtual Key simulation if WinRT did not handle it
        if !executed_via_api {
            debug!("Using Win32 virtual key fallback for action: {}", action);
            unsafe {
                match action {
                    "play" | "pause" | "toggle" => {
                        keybd_event(VK_MEDIA_PLAY_PAUSE.0 as u8, 0, Default::default(), 0);
                        keybd_event(VK_MEDIA_PLAY_PAUSE.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                    }
                    "next" => {
                        keybd_event(VK_MEDIA_NEXT_TRACK.0 as u8, 0, Default::default(), 0);
                        keybd_event(VK_MEDIA_NEXT_TRACK.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                    }
                    "prev" => {
                        keybd_event(VK_MEDIA_PREV_TRACK.0 as u8, 0, Default::default(), 0);
                        keybd_event(VK_MEDIA_PREV_TRACK.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                    }
                    _ => {}
                }
            }
        }

        // Allow Windows audio session 90ms to propagate new state
        std::thread::sleep(std::time::Duration::from_millis(90));
        get_current_media_state()
    }
}

#[cfg(not(target_os = "windows"))]
mod non_win {
    use super::MediaState;
    pub fn get_current_media_state() -> Option<MediaState> {
        None
    }
    pub fn send_media_command(_action: &str) -> Option<MediaState> {
        None
    }
}

#[cfg(target_os = "windows")]
pub use win::*;

#[cfg(not(target_os = "windows"))]
pub use non_win::*;
