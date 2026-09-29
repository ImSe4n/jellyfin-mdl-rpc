use crate::ClientBuilder;

#[test]
fn build_client_error() {
    let client = ClientBuilder::new().build();

    if let Ok(_) = client {
        panic!("client was constructed even though required values are missing!");
    }
}

#[test]
fn invalid_url() {
    let mut builder = ClientBuilder::new();
    builder
        .api_key("a1b2c3d4")
        .username("test")
        .url("url_without_base.com");

    let client = builder.build();

    if let Ok(_) = client {
        panic!("client constructed without a valid url!")
    }
}

#[test]
fn profile_card_reuses_media_image_time_bar_and_paused_badge() {
    use crate::{Button, Client, ProfileCard};
    use discord_rich_presence::activity::{Assets, Timestamps};

    let card = ProfileCard {
        details: "MyDramaList · ImSe4n".to_string(),
        state: "43 completed · 1 watching · 56 planned".to_string(),
        image_text: Some("728 episodes".to_string()),
        buttons: vec![
            Button::new("MDL Profile".to_string(), "https://example.com/p".to_string()),
            Button::new("Watchlist".to_string(), "https://example.com/l".to_string()),
        ],
    };
    let media_assets = Assets::new()
        .large_image("https://i.imgur.com/poster.jpeg")
        .large_text("Episode title")
        .small_image("https://i.imgur.com/wlHSvYy.png")
        .small_text("Paused");
    let media_timestamps = Timestamps::new().start(100).end(200);

    let activity = Client::profile_card_activity(&card, media_assets, media_timestamps);
    let json = serde_json::to_value(&activity).unwrap();

    assert_eq!(json["details"], "MyDramaList · ImSe4n");
    assert_eq!(json["state"], "43 completed · 1 watching · 56 planned");
    assert_eq!(json["assets"]["large_image"], "https://i.imgur.com/poster.jpeg");
    assert_eq!(json["assets"]["large_text"], "728 episodes");
    assert_eq!(json["assets"]["small_text"], "Paused");
    assert_eq!(json["timestamps"]["start"], 100);
    assert_eq!(json["timestamps"]["end"], 200);
    assert_eq!(json["buttons"].as_array().unwrap().len(), 2);
}
