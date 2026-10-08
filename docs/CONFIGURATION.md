# Configuration

Everything goes in one JSON file. This guide covers the required Jellyfin part,
then the optional MyDramaList part.

## 1. Where the config file goes

| OS | Default path |
|---|---|
| Windows | `%APPDATA%\jellyfin-rpc\main.json` |
| Linux / macOS | `~/.config/jellyfin-rpc/main.json` (or `$XDG_CONFIG_HOME/jellyfin-rpc/main.json`) |

Create the `jellyfin-rpc` folder if it doesn't exist. To keep the file somewhere
else, pass its path with `-c`:

```sh
jellyfin-rpc -c /path/to/main.json
```

Copy [`example.json`](../example.json) there as a starting point. It's plain JSON,
so no comments and no trailing commas.

> **Delete the `_comment` and `blacklist` lines from `example.json` before you use
> it.** The example's blacklist hides every media type, so you'd see nothing at all.

## 2. Jellyfin (required)

```json
"jellyfin": {
    "url": "https://jellyfin.example.com",
    "api_key": "YOUR_JELLYFIN_API_KEY",
    "username": "your_jellyfin_username"
}
```

| Key | What it is |
|---|---|
| `url` | Your server's address, the same one you open in a browser, e.g. `http://192.168.1.10:8096`. No trailing `/web`. |
| `api_key` | In Jellyfin: **Dashboard → API Keys → +**, name it anything, and copy the key. |
| `username` | Your Jellyfin username (not your MDL one). For several accounts, use a list: `["alice", "bob"]`. |

That's enough to get a working now-playing status. Optional Jellyfin keys:

| Key | Default | What it does |
|---|---|---|
| `self_signed_cert` | `false` | Set to `true` if your server uses a self-signed HTTPS certificate. |
| `show_simple` | `false` | Episodes show `S1E3` with no episode title. |
| `append_prefix` | `false` | Pads numbers: `S01E03`. |
| `add_divider` | `false` | Adds a dash: `S1 - E3`. |
| `music` / `movies` / `episodes` | | `display` (e.g. `["genres"]`) and `separator` control the extra info line. |
| `blacklist` | | `media_types` (`music`, `movie`, `episode`, `livetv`, `book`, `audiobook`) and `libraries` (library names) to never show. **Leave it out entirely** if you don't want to hide anything. |

The upstream [setup wiki](https://github.com/Radiicall/jellyfin-rpc/wiki/Setup)
covers the display templates in more detail.

## 3. Discord and images (optional)

```json
"discord": {
    "show_paused": true
},
"images": {
    "enable_images": true,
    "litterbox_images": true
}
```

- `discord.show_paused`: keep showing your status while paused. Defaults to `true`.
- `discord.application_id`: only change this if you've made your own Discord app.
  Leave it out to use the default.
- `discord.buttons`: custom buttons for the **now-playing** card. The MDL cards
  always use their own two buttons.
- `images`: posters need a public URL that Discord can load. `litterbox_images`
  needs no account. `imgur_images` needs an Imgur `client_id` in an `"imgur"`
  section. With images off, the cards still work, just without a poster.

## 4. MyDramaList (optional)

### 4a. Check your MDL profile is public

Open `https://mydramalist.com/profile/YOUR_MDL_USERNAME` in a private browser
window. If you can see your stats and drama list without logging in, you're set.
If not, change your privacy settings on MyDramaList.

Your username is the part after `/profile/` in that URL. Usernames use letters,
digits, `_`, `.` and `-`. Anything else gets rejected and the MDL cards stay off.

### 4b. Install Python and curl_cffi

```sh
pip install -r mdl/requirements.txt
```

Then find the **full path** of that Python. Jellyfin-RPC runs it directly, so a
bare `python` might resolve to a different interpreter, or on Windows to the
Microsoft Store shortcut.

```sh
python -c "import sys, curl_cffi; print(sys.executable)"
```

If that prints an error, `curl_cffi` isn't installed for that Python. Whatever path
it prints goes in `python` below.

### 4c. Test the fetch script by hand

```sh
python mdl/mdl_fetch.py YOUR_MDL_USERNAME mdl_stats.json
```

It should print something like `43 completed, 1 watching, 80 planned, 728 episodes`
and create `mdl_stats.json`. If it fails, fix that first (see
[Troubleshooting](#troubleshooting)). The RPC runs this same command.

### 4d. Add the `mdl` section

Use **absolute paths**. Relative ones resolve from wherever you launched
Jellyfin-RPC, which can change (for example, when it's a service).

Windows:

```json
"mdl": {
    "username": "YOUR_MDL_USERNAME",
    "python": "C:\\Users\\you\\AppData\\Local\\Programs\\Python\\Python312\\python.exe",
    "script": "C:\\path\\to\\jellyfin-mdl-rpc\\mdl\\mdl_fetch.py",
    "stats_file": "C:\\Users\\you\\AppData\\Roaming\\jellyfin-rpc\\mdl_stats.json",
    "switch_seconds": 30,
    "refresh_hours": 6
}
```

Linux / macOS:

```json
"mdl": {
    "username": "YOUR_MDL_USERNAME",
    "python": "/usr/bin/python3",
    "script": "/home/you/jellyfin-mdl-rpc/mdl/mdl_fetch.py",
    "stats_file": "/home/you/.config/jellyfin-rpc/mdl_stats.json",
    "switch_seconds": 30,
    "refresh_hours": 6
}
```

In JSON, Windows backslashes have to be doubled (`\\`). Forward slashes
(`C:/Users/you/...`) also work.

| Key | Required | Default | What it does |
|---|---|---|---|
| `username` | yes | | Your MyDramaList username. |
| `python` | yes | | Full path to the Python that has `curl_cffi`. |
| `script` | yes | | Full path to `mdl/mdl_fetch.py` in your clone. |
| `stats_file` | yes | | Where the stats JSON is written. Its folder must already exist. |
| `switch_seconds` | no | `30` | How long each card stays up. Minimum `5`. |
| `refresh_hours` | no | `6` | How often to re-fetch MDL. Decimals like `0.5` are allowed; the minimum is `0.5`. |

`refresh_hours` has a minimum because MyDramaList is behind Cloudflare, and
fetching too often could get you blocked. Your stats don't change by the minute, so
the default of 6 hours is plenty.

To turn the MDL cards off, delete the whole `mdl` section.

## 5. A complete example

```json
{
    "jellyfin": {
        "url": "https://jellyfin.example.com",
        "api_key": "YOUR_JELLYFIN_API_KEY",
        "username": "your_jellyfin_username"
    },
    "discord": {
        "show_paused": true
    },
    "images": {
        "enable_images": true,
        "litterbox_images": true
    },
    "mdl": {
        "username": "YOUR_MDL_USERNAME",
        "python": "/usr/bin/python3",
        "script": "/home/you/jellyfin-mdl-rpc/mdl/mdl_fetch.py",
        "stats_file": "/home/you/.config/jellyfin-rpc/mdl_stats.json"
    }
}
```

## Checking that it works

Run it with debug logging:

```sh
jellyfin-rpc -v debug
```

At startup you should see:

```
MyDramaList card enabled for YOUR_MDL_USERNAME, switching every 30s
MyDramaList stats refreshed: 43 completed, 1 watching, 80 planned, 728 episodes
```

Then play something on Jellyfin and wait one `switch_seconds`.

## Troubleshooting

| Log message | Cause and fix |
|---|---|
| `mdl.username "..." is not a valid MyDramaList username` | There's a typo or a disallowed character in the username. Copy it from your profile URL. |
| `Could not run MyDramaList fetch script with ...` | The `python` path is wrong. Re-run the command in step 4b and paste the exact path it prints. |
| `mdl_fetch failed: No module named 'curl_cffi'` | `curl_cffi` is installed for a different Python than the one in `python`. Run `<your python path> -m pip install curl_cffi`. |
| `mdl_fetch failed: unexpected page title "Just a moment..."` | Cloudflare blocked the request. Update `curl_cffi` (`pip install -U curl_cffi`) and try again later. Don't lower `refresh_hours` to retry faster. |
| `mdl_fetch failed: unexpected page title ...` (anything else) | The profile is private, the username is wrong, or MDL changed its page layout. Check step 4a. |
| `GET ... returned HTTP <code>` | MDL returned an error page. A 404 usually means the username doesn't exist; a 403 or 503 is usually Cloudflare (see above). |
| `No MyDramaList stats at ...` | The fetch hasn't succeeded yet, or `stats_file` points to a different file than the one being written. Only the now-playing card shows until it's fixed. |
| MDL cards never appear, and no MDL lines show in the log | The `mdl` section is missing, misspelled, or in the wrong place. It goes at the top level, next to `jellyfin`, not inside it. |

When a fetch fails, the last good stats file is kept, so the cards keep showing
your older numbers instead of disappearing.
