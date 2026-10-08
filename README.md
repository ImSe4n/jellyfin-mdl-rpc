# Jellyfin-MDL-RPC

Discord Rich Presence for Jellyfin, with your **MyDramaList** profile built in.

While you watch something on Jellyfin, your Discord status shows it as usual. Every
so often it switches to a card with your MyDramaList stats, then back to what's
playing. Each card has buttons that link to your MDL profile and watchlist.

This is a fork of [Radiicall/jellyfin-rpc](https://github.com/Radiicall/jellyfin-rpc),
which has been archived. All the original Jellyfin features still work, and the
MyDramaList part is optional.

## What the MDL cards show

The cards rotate in this order while something is playing:

| Card | Example |
|---|---|
| Now playing | The normal Jellyfin card: title, episode, time bar, poster |
| Overview | `MyDramaList · YourName` / `43 completed · 1 watching · 80 planned` |
| Watch time | `Watched 28d 10h 28m of dramas` / `728 episodes · 42 shows · 2 movies (3h 57m)` |
| Ratings | `Average rating 8.4/10` / `Top rated (10.0/10): Some Drama +2 more` |
| Countries | `Completed by country` / `South Korea: 30 · Japan: 8 · China: 5` |

A card with nothing to show (for example, no rated dramas yet) gets skipped. The
MDL cards keep the poster, the time bar, and the paused badge from the media you're
watching. Hovering the poster shows the media title.

The MDL cards only show while something is playing. When playback stops, your status
clears like it normally would.

## How it works

- `mdl/mdl_fetch.py` loads your **public** MDL profile and drama list pages, the
  same pages anyone can open in a browser, and writes the numbers to a JSON file.
  It's read-only: it never logs in and never changes anything on MyDramaList.
- MyDramaList is behind Cloudflare, so the script uses
  [`curl_cffi`](https://github.com/lexiforest/curl_cffi) to look like a normal
  Chrome browser.
- Jellyfin-RPC runs the script in the background (every 6 hours by default) and
  builds the cards from that JSON file. A slow or failed fetch never holds up your
  presence. It keeps using the last good stats.
- If the `mdl` section of your config is missing or broken, only the MDL cards
  turn off. Everything else keeps working.

## Requirements

- A Jellyfin server and an API key for it
- Discord desktop app, open and logged in, on the same machine
- [Rust](https://rustup.rs) to build it (there are no prebuilt releases with the MDL
  feature yet)
- **For the MDL cards:** Python 3.8+ with `curl_cffi`, and a **public** MyDramaList
  profile

## Quick start

```sh
git clone https://github.com/ImSe4n/jellyfin-mdl-rpc.git
cd jellyfin-mdl-rpc
cargo build --release
pip install -r mdl/requirements.txt
```

Then write your config. **[docs/CONFIGURATION.md](docs/CONFIGURATION.md) walks
through every step**: where the file goes, getting a Jellyfin API key, and
setting up the `mdl` section. [`example.json`](example.json) is a starting point.

Run it:

```sh
./target/release/jellyfin-rpc            # Linux / macOS
.\target\release\jellyfin-rpc.exe        # Windows
```

Start a drama on Jellyfin, and after 30 seconds your status switches to the MDL
overview card.

## Running the tests

```sh
cargo test --workspace
cd mdl && python -m unittest discover -s tests
```

The Python tests run against saved MDL pages in `mdl/tests/fixtures/`, so they
don't need network access or `curl_cffi`.

## Credits and license

Built on [Jellyfin-RPC](https://github.com/Radiicall/jellyfin-rpc) by Radical.
The upstream README is at [jellyfin-rpc-cli/README.md](jellyfin-rpc-cli/README.md),
and its [wiki](https://github.com/Radiicall/jellyfin-rpc/wiki) still covers the
Jellyfin side.

Not affiliated with MyDramaList, Jellyfin, or Discord.

Licensed under [GPL-3.0](LICENSE), the same as upstream.
