//! Turns the stats file written by `mdl_fetch.py` into the Discord cards that rotate
//! with the now-playing card. Each card builder returns `None` when it has nothing
//! worth showing, and that card is skipped.

use jellyfin_rpc::{Button, ProfileCard};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Discord rejects activity text longer than this.
const MAX_TEXT_CHARS: usize = 128;

/// Stats file written by `mdl_fetch.py`. Unknown fields are ignored, and fields added
/// after the first version default, so a stats file from an older fetch still loads.
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
    #[serde(default)]
    pub shows: u32,
    #[serde(default)]
    pub movies: u32,
    #[serde(default)]
    pub movie_time: Option<String>,
    #[serde(default)]
    pub mean_score: Option<f64>,
    #[serde(default)]
    pub top_rated: Option<TopRated>,
    #[serde(default)]
    pub countries: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TopRated {
    pub score: f64,
    /// Every title tied for the top score.
    pub titles: Vec<String>,
}

pub fn load_stats(path: &str) -> Result<MdlStats, Box<dyn std::error::Error>> {
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

/// Every card worth showing right now, in rotation order.
pub fn profile_cards(stats: &MdlStats) -> Vec<ProfileCard> {
    [
        Some(overview_card(stats)),
        watch_time_card(stats),
        ratings_card(stats),
        countries_card(stats),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn card(
    stats: &MdlStats,
    details: String,
    state: String,
    image_text: Option<String>,
) -> ProfileCard {
    ProfileCard {
        details: clip(details),
        state: clip(state),
        image_text: image_text.map(clip),
        buttons: vec![
            Button::new("MDL Profile".to_string(), stats.profile_url.clone()),
            Button::new("Watchlist".to_string(), stats.list_url.clone()),
        ],
    }
}

fn clip(text: String) -> String {
    if text.chars().count() <= MAX_TEXT_CHARS {
        return text;
    }
    let mut clipped: String = text.chars().take(MAX_TEXT_CHARS - 1).collect();
    clipped.push('…');
    clipped
}

pub fn overview_card(stats: &MdlStats) -> ProfileCard {
    card(
        stats,
        format!("MyDramaList · {}", stats.username),
        format!(
            "{} completed · {} watching · {} planned",
            stats.completed, stats.watching, stats.plan_to_watch
        ),
        None,
    )
}

fn watch_time_card(stats: &MdlStats) -> Option<ProfileCard> {
    let show_time = stats.show_time.as_ref()?;
    let movies = match &stats.movie_time {
        Some(movie_time) if stats.movies > 0 => format!("{} movies ({movie_time})", stats.movies),
        _ => format!("{} movies", stats.movies),
    };
    Some(card(
        stats,
        format!("Watched {show_time} of dramas"),
        format!("{} episodes · {} shows · {movies}", stats.episodes, stats.shows),
        None,
    ))
}

fn ratings_card(stats: &MdlStats) -> Option<ProfileCard> {
    let mean = stats.mean_score?;
    let top = stats.top_rated.as_ref()?;
    let state = match top.titles.split_first() {
        Some((first, [])) => format!("Top rated ({:.1}/10): {first}", top.score),
        Some((first, rest)) => {
            format!("Top rated ({:.1}/10): {first} +{} more", top.score, rest.len())
        }
        None => return None,
    };
    Some(card(stats, format!("Average rating {mean:.1}/10"), state, None))
}

fn countries_card(stats: &MdlStats) -> Option<ProfileCard> {
    let mut countries: Vec<(&String, &u32)> = stats.countries.iter().collect();
    if countries.is_empty() {
        return None;
    }
    // Most-watched first; the map already orders ties by name.
    countries.sort_by(|a, b| b.1.cmp(a.1));
    let state = countries
        .iter()
        .map(|(country, count)| format!("{country}: {count}"))
        .collect::<Vec<_>>()
        .join(" · ");
    Some(card(stats, "Completed by country".to_string(), state, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> MdlStats {
        MdlStats {
            username: "ExampleUser".to_string(),
            profile_url: "https://mydramalist.com/profile/ExampleUser".to_string(),
            list_url: "https://mydramalist.com/dramalist/ExampleUser".to_string(),
            watching: 1,
            completed: 43,
            plan_to_watch: 56,
            episodes: 728,
            show_time: Some("28d 10h 28m".to_string()),
            shows: 42,
            movies: 2,
            movie_time: Some("3h 57m".to_string()),
            mean_score: Some(8.98),
            top_rated: Some(TopRated {
                score: 10.0,
                titles: vec!["Go Ahead".to_string(), "Queen of Tears".to_string()],
            }),
            countries: BTreeMap::from([
                ("China".to_string(), 11),
                ("South Korea".to_string(), 32),
            ]),
        }
    }

    #[test]
    fn overview_shows_counts_and_links() {
        let card = overview_card(&stats());
        assert_eq!(card.details, "MyDramaList · ExampleUser");
        assert_eq!(card.state, "43 completed · 1 watching · 56 planned");
        // Leaves the hover text to the media card.
        assert_eq!(card.image_text, None);
        assert_eq!(card.buttons.len(), 2);
        assert_eq!(card.buttons[0].url, "https://mydramalist.com/profile/ExampleUser");
        assert_eq!(card.buttons[1].url, "https://mydramalist.com/dramalist/ExampleUser");
    }

    #[test]
    fn all_cards_in_rotation_order() {
        let details: Vec<String> = profile_cards(&stats())
            .into_iter()
            .map(|card| card.details)
            .collect();
        assert_eq!(
            details,
            [
                "MyDramaList · ExampleUser",
                "Watched 28d 10h 28m of dramas",
                "Average rating 9.0/10",
                "Completed by country",
            ]
        );
    }

    #[test]
    fn watch_time_card_counts_episodes_shows_and_movies() {
        let card = watch_time_card(&stats()).unwrap();
        assert_eq!(card.state, "728 episodes · 42 shows · 2 movies (3h 57m)");
        assert_eq!(card.image_text, None);
    }

    #[test]
    fn watch_time_card_without_movies() {
        let card = watch_time_card(&MdlStats {
            movies: 0,
            movie_time: None,
            ..stats()
        })
        .unwrap();
        assert_eq!(card.state, "728 episodes · 42 shows · 0 movies");
    }

    #[test]
    fn ratings_card_mentions_ties() {
        let card = ratings_card(&stats()).unwrap();
        assert_eq!(card.state, "Top rated (10.0/10): Go Ahead +1 more");
    }

    #[test]
    fn ratings_card_with_one_top_title() {
        let card = ratings_card(&MdlStats {
            top_rated: Some(TopRated {
                score: 9.5,
                titles: vec!["Go Ahead".to_string()],
            }),
            ..stats()
        })
        .unwrap();
        assert_eq!(card.state, "Top rated (9.5/10): Go Ahead");
    }

    #[test]
    fn countries_card_lists_most_watched_first() {
        let card = countries_card(&stats()).unwrap();
        assert_eq!(card.state, "South Korea: 32 · China: 11");
    }

    #[test]
    fn cards_without_data_are_skipped() {
        let bare = MdlStats {
            show_time: None,
            mean_score: None,
            top_rated: None,
            countries: BTreeMap::new(),
            ..stats()
        };
        assert_eq!(profile_cards(&bare).len(), 1);
    }

    #[test]
    fn long_text_is_clipped_for_discord() {
        let long = "x".repeat(200);
        let card = card(&stats(), long.clone(), long, None);
        assert_eq!(card.details.chars().count(), MAX_TEXT_CHARS);
        assert!(card.state.ends_with('…'));
    }

    #[test]
    fn parses_old_stats_file_without_new_fields() {
        let json = r#"{"username":"ExampleUser","profile_url":"p","list_url":"l",
            "fetched_at":1,"episodes":728,"shows":42,"movies":2,
            "show_time":"28d 10h 28m","movie_time":"3h 57m","avatar_url":null,
            "dropped":0,"watching":1,"plan_to_watch":56,"completed":43,"on_hold":0}"#;
        let parsed: MdlStats = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.completed, 43);
        assert_eq!(parsed.mean_score, None);
        assert!(parsed.countries.is_empty());
    }

    #[test]
    fn parses_new_stats_file() {
        let json = r#"{"username":"ExampleUser","profile_url":"p","list_url":"l",
            "watching":1,"completed":43,"plan_to_watch":56,"episodes":728,
            "show_time":"28d 10h 28m","show_minutes":40948,"mean_score":8.98,
            "rated":43,"top_rated":{"score":10.0,"titles":["Go Ahead"]},
            "countries":{"South Korea":32,"China":11}}"#;
        let parsed: MdlStats = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.countries["South Korea"], 32);
        assert_eq!(parsed.top_rated.unwrap().titles, ["Go Ahead"]);
    }
}
