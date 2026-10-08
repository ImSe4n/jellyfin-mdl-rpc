//! MyDramaList profile cards: refreshes the stats written by `mdl_fetch.py` and
//! decides when to show a card instead of the now-playing card.
//!
//! Read-only. Nothing here writes to MyDramaList.

use log::{info, warn};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

mod cards;
pub use cards::{load_stats, profile_cards};

/// Presence is pushed every loop tick anyway, so this only guards against a card
/// flickering past before anyone can read it.
pub const MIN_SWITCH_SECONDS: u64 = 5;
pub const DEFAULT_SWITCH_SECONDS: u64 = 30;
/// MDL is behind Cloudflare; fetching rarely keeps us well clear of any blocking.
pub const MIN_REFRESH_HOURS: f64 = 0.5;
pub const DEFAULT_REFRESH_HOURS: f64 = 6.0;

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

/// MDL usernames are letters, digits and `_ . -`; anything else is a config mistake.
pub fn is_valid_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 64
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Counts how many intervals the current playback has been running. Each step is
/// one card; `card_for_step` turns it into the card to show. Resets to step 0 (the
/// media card) when playback stops.
pub struct Rotation {
    interval: Duration,
    step: usize,
    since: Option<Instant>,
}

impl Rotation {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            step: 0,
            since: None,
        }
    }

    pub fn update(&mut self, now: Instant, playing: bool) -> usize {
        if !playing {
            self.step = 0;
            self.since = None;
            return self.step;
        }

        let since = *self.since.get_or_insert(now);
        if now.duration_since(since) >= self.interval {
            self.step += 1;
            self.since = Some(now);
        }
        self.step
    }
}

/// Cycles the media card and every profile card in turn: media, card 1, card 2, ...,
/// card n, media, card 1, ... `None` means the media card; `Some(i)` is profile card `i`.
pub fn card_for_step(step: usize, card_count: usize) -> Option<usize> {
    match step % (card_count + 1) {
        0 => None,
        position => Some(position - 1),
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

    #[test]
    fn username_validation() {
        assert!(is_valid_username("ExampleUser"));
        assert!(is_valid_username("a.b_c-d"));
        for bad in ["", "../x", "a b", "a/b", "a?b"] {
            assert!(!is_valid_username(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rotation_steps_once_per_interval() {
        let interval = Duration::from_secs(30);
        let mut rotation = Rotation::new(interval);
        let start = Instant::now();

        assert_eq!(rotation.update(start, true), 0);
        assert_eq!(rotation.update(start + Duration::from_secs(29), true), 0);
        assert_eq!(rotation.update(start + interval, true), 1);
        assert_eq!(rotation.update(start + interval * 2, true), 2);
    }

    #[test]
    fn rotation_resets_to_media_when_playback_stops() {
        let interval = Duration::from_secs(30);
        let mut rotation = Rotation::new(interval);
        let start = Instant::now();

        rotation.update(start, true);
        assert_eq!(rotation.update(start + interval, true), 1);
        assert_eq!(rotation.update(start + interval, false), 0);
        // New playback starts on the media card again, for a full interval.
        let resumed = start + interval * 3;
        assert_eq!(rotation.update(resumed, true), 0);
        assert_eq!(rotation.update(resumed + Duration::from_secs(29), true), 0);
        assert_eq!(rotation.update(resumed + interval, true), 1);
    }

    #[test]
    fn every_profile_card_shows_before_the_media_card_returns() {
        let cards: Vec<Option<usize>> = (0..8).map(|step| card_for_step(step, 3)).collect();
        assert_eq!(
            cards,
            [None, Some(0), Some(1), Some(2), None, Some(0), Some(1), Some(2)]
        );
    }

    #[test]
    fn no_profile_cards_means_media_only() {
        assert!((0..4).all(|step| card_for_step(step, 0).is_none()));
    }
}
