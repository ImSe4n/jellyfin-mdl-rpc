//! MyDramaList profile card: loads stats written by `mdl_fetch.py`, turns them into a
//! Discord card, and decides when to show it instead of the now-playing card.
//!
//! Read-only. Nothing here writes to MyDramaList.

use jellyfin_rpc::{Button, ProfileCard};
use log::{info, warn};
use serde::Deserialize;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

/// Discord rate-limits presence updates; switching faster than this gains nothing.
pub const MIN_SWITCH_SECONDS: u64 = 15;
pub const DEFAULT_SWITCH_SECONDS: u64 = 30;
/// MDL is behind Cloudflare; fetching rarely keeps us well clear of any blocking.
pub const MIN_REFRESH_HOURS: u64 = 1;
pub const DEFAULT_REFRESH_HOURS: u64 = 6;

/// Resolved `mdl` section of the config.
#[derive(Debug, Clone, PartialEq)]
pub struct MdlConfig {
    pub username: String,
    pub python: String,
    pub script: String,
    pub stats_file: String,
    pub switch_interval: Duration,
    pub refresh_interval: Duration,
}

/// Stats file written by `mdl_fetch.py`. Unknown fields are ignored.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MdlStats {
    pub username: String,
    pub profile_url: String,
    pub list_url: String,
    pub watching: u32,
    pub completed: u32,
    pub plan_to_watch: u32,
    pub episodes: u32,
    pub show_time: Option<String>,
}

/// MDL usernames are letters, digits and `_ . -`; anything else is a config mistake.
pub fn is_valid_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 64
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub fn load_stats(path: &str) -> Result<MdlStats, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub fn card_from_stats(stats: &MdlStats) -> ProfileCard {
    let image_text = match &stats.show_time {
        Some(time) => format!("{} episodes · {} watched", stats.episodes, time),
        None => format!("{} episodes", stats.episodes),
    };

    ProfileCard {
        details: format!("MyDramaList · {}", stats.username),
        state: format!(
            "{} completed · {} watching · {} planned",
            stats.completed, stats.watching, stats.plan_to_watch
        ),
        image_text: Some(image_text),
        buttons: vec![
            Button::new("MDL Profile".to_string(), stats.profile_url.clone()),
            Button::new("Watchlist".to_string(), stats.list_url.clone()),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Media,
    Profile,
}

/// Alternates between the media card and the profile card while something plays.
/// Always starts on the media card, and resets to it when playback stops.
pub struct Rotation {
    interval: Duration,
    phase: Phase,
    since: Option<Instant>,
}

impl Rotation {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            phase: Phase::Media,
            since: None,
        }
    }

    pub fn update(&mut self, now: Instant, playing: bool) -> Phase {
        if !playing {
            self.phase = Phase::Media;
            self.since = None;
            return self.phase;
        }

        let since = *self.since.get_or_insert(now);
        if now.duration_since(since) >= self.interval {
            self.phase = match self.phase {
                Phase::Media => Phase::Profile,
                Phase::Profile => Phase::Media,
            };
            self.since = Some(now);
        }
        self.phase
    }
}

/// Runs `mdl_fetch.py` now and then every `refresh_interval`, on a background thread
/// so a slow fetch never delays presence updates.
pub fn spawn_refresher(config: MdlConfig) {
    thread::spawn(move || loop {
        run_fetch(&config);
        thread::sleep(config.refresh_interval);
    });
}

fn run_fetch(config: &MdlConfig) {
    let mut command = Command::new(&config.python);
    command
        .arg(&config.script)
        .arg(&config.username)
        .arg(&config.stats_file);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    match command.output() {
        Ok(output) if output.status.success() => {
            info!(
                "MyDramaList stats refreshed: {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
        Ok(output) => warn!(
            "MyDramaList fetch failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(err) => warn!(
            "Could not run MyDramaList fetch script with {}: {}",
            config.python, err
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> MdlStats {
        MdlStats {
            username: "ImSe4n".to_string(),
            profile_url: "https://mydramalist.com/profile/ImSe4n".to_string(),
            list_url: "https://mydramalist.com/dramalist/ImSe4n".to_string(),
            watching: 1,
            completed: 43,
            plan_to_watch: 56,
            episodes: 728,
            show_time: Some("28d 10h 28m".to_string()),
        }
    }

    #[test]
    fn card_shows_counts_and_links() {
        let card = card_from_stats(&stats());
        assert_eq!(card.details, "MyDramaList · ImSe4n");
        assert_eq!(card.state, "43 completed · 1 watching · 56 planned");
        assert_eq!(
            card.image_text.as_deref(),
            Some("728 episodes · 28d 10h 28m watched")
        );
        assert_eq!(card.buttons.len(), 2);
        assert_eq!(card.buttons[0].url, "https://mydramalist.com/profile/ImSe4n");
        assert_eq!(card.buttons[1].url, "https://mydramalist.com/dramalist/ImSe4n");
    }

    #[test]
    fn card_without_watch_time() {
        let card = card_from_stats(&MdlStats {
            show_time: None,
            ..stats()
        });
        assert_eq!(card.image_text.as_deref(), Some("728 episodes"));
    }

    #[test]
    fn parses_stats_file_from_fetch_script() {
        let json = r#"{"username":"ImSe4n","profile_url":"p","list_url":"l",
            "fetched_at":1,"episodes":728,"shows":42,"movies":2,
            "show_time":"28d 10h 28m","movie_time":"3h 57m","avatar_url":null,
            "dropped":0,"watching":1,"plan_to_watch":56,"completed":43,"on_hold":0}"#;
        let parsed: MdlStats = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.completed, 43);
        assert_eq!(parsed.episodes, 728);
    }

    #[test]
    fn username_validation() {
        assert!(is_valid_username("ImSe4n"));
        assert!(is_valid_username("a.b_c-d"));
        for bad in ["", "../x", "a b", "a/b", "a?b"] {
            assert!(!is_valid_username(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rotation_starts_on_media_and_alternates() {
        let interval = Duration::from_secs(30);
        let mut rotation = Rotation::new(interval);
        let start = Instant::now();

        assert_eq!(rotation.update(start, true), Phase::Media);
        assert_eq!(rotation.update(start + Duration::from_secs(29), true), Phase::Media);
        assert_eq!(rotation.update(start + interval, true), Phase::Profile);
        assert_eq!(rotation.update(start + interval * 2, true), Phase::Media);
    }

    #[test]
    fn rotation_resets_to_media_when_playback_stops() {
        let interval = Duration::from_secs(30);
        let mut rotation = Rotation::new(interval);
        let start = Instant::now();

        rotation.update(start, true);
        assert_eq!(rotation.update(start + interval, true), Phase::Profile);
        assert_eq!(rotation.update(start + interval, false), Phase::Media);
        // New playback starts on the media card again, for a full interval.
        let resumed = start + interval * 3;
        assert_eq!(rotation.update(resumed, true), Phase::Media);
        assert_eq!(rotation.update(resumed + Duration::from_secs(29), true), Phase::Media);
    }
}
