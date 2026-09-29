"""Fetch public MyDramaList profile stats and write them to a JSON file.

Read-only: it loads the public profile and drama list pages, the same pages
anyone can open in a browser. MDL sits behind Cloudflare, which rejects plain
HTTP clients, so requests go through curl_cffi impersonating Chrome.

Usage: mdl_fetch.py <username> <output.json>
Exit code is non-zero on any failure; the previous output file is left intact.
"""

import html
import json
import os
import re
import sys
import tempfile
import time

BASE_URL = "https://mydramalist.com"
REQUEST_TIMEOUT_SECONDS = 20
USERNAME_PATTERN = re.compile(r"^[A-Za-z0-9_.-]{1,64}$")

# Status labels as they appear in the drama list table, mapped to output keys.
STATUS_KEYS = {
    "Watching": "watching",
    "Completed": "completed",
    "Plan to Watch": "plan_to_watch",
    "On-Hold": "on_hold",
    "On Hold": "on_hold",
    "Dropped": "dropped",
}


class ParseError(Exception):
    """The page was not the MDL page we expected (e.g. a Cloudflare challenge)."""


def validate_username(username: str) -> str:
    if not USERNAME_PATTERN.match(username):
        raise ValueError(f"invalid MyDramaList username: {username!r}")
    return username


def _require_title(page: str, expected: str) -> None:
    match = re.search(r"<title>([^<]*)</title>", page)
    title = html.unescape(match.group(1)) if match else ""
    if expected not in title:
        raise ParseError(f"unexpected page title {title!r}, wanted {expected!r}")


def _duration_text(block: str) -> str:
    """Turn '<b>28</b>d <b>10</b>h <b>28</b>m' into '28d 10h 28m'."""
    text = re.sub(r"<[^>]+>", "", block)
    return " ".join(text.split())


def parse_profile(page: str, username: str) -> dict:
    _require_title(page, f"{username}'s Profile")

    profile = {
        "episodes": 0,
        "shows": 0,
        "movies": 0,
        "show_time": None,
        "movie_time": None,
        "avatar_url": None,
    }

    # Each "ALL TIME" stat block: <h6>duration</h6> ... <div class="text-muted">summary</div>
    for duration, summary in re.findall(
        r"<h6[^>]*>(.*?)</h6>(?:(?!<h6).)*?<div class=\"text-muted\">([^<]*)</div>",
        page,
        flags=re.S,
    ):
        episodes = re.search(r"(\d+) episodes?", summary)
        shows = re.search(r"(\d+) shows?", summary)
        movies = re.search(r"(\d+) movies?", summary)
        if episodes or shows:
            profile["episodes"] = int(episodes.group(1)) if episodes else 0
            profile["shows"] = int(shows.group(1)) if shows else 0
            profile["show_time"] = _duration_text(duration)
        elif movies:
            profile["movies"] = int(movies.group(1))
            profile["movie_time"] = _duration_text(duration)

    avatar = re.search(
        r'<img class="img-responsive mdl-rounded" src="(https://[^"]+)" alt="'
        + re.escape(username)
        + '"',
        page,
    )
    if avatar:
        profile["avatar_url"] = avatar.group(1)

    return profile


def parse_dramalist(page: str, username: str) -> dict:
    _require_title(page, f"{username}'s Drama List")

    counts = {key: 0 for key in set(STATUS_KEYS.values())}
    for label in re.findall(r'<td class="msv2-i-status">([^<]*)</td>', page):
        key = STATUS_KEYS.get(html.unescape(label).strip())
        if key:
            counts[key] += 1
    return counts


def build_stats(username: str, profile_page: str, list_page: str) -> dict:
    return {
        "username": username,
        "profile_url": f"{BASE_URL}/profile/{username}",
        "list_url": f"{BASE_URL}/dramalist/{username}",
        "fetched_at": int(time.time()),
        **parse_profile(profile_page, username),
        **parse_dramalist(list_page, username),
    }


def fetch_page(url: str) -> str:
    from curl_cffi import requests  # imported here so tests don't need it

    response = requests.get(url, impersonate="chrome", timeout=REQUEST_TIMEOUT_SECONDS)
    if response.status_code != 200:
        raise RuntimeError(f"GET {url} returned HTTP {response.status_code}")
    return response.text


def write_atomically(path: str, data: dict) -> None:
    directory = os.path.dirname(os.path.abspath(path))
    fd, tmp_path = tempfile.mkstemp(dir=directory, prefix=".mdl_stats_", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as tmp:
            json.dump(data, tmp, indent=2)
        os.replace(tmp_path, path)
    except BaseException:
        os.unlink(tmp_path)
        raise


def main(argv: list) -> int:
    if len(argv) != 3:
        print("usage: mdl_fetch.py <username> <output.json>", file=sys.stderr)
        return 2

    try:
        username = validate_username(argv[1])
        profile_page = fetch_page(f"{BASE_URL}/profile/{username}")
        list_page = fetch_page(f"{BASE_URL}/dramalist/{username}")
        stats = build_stats(username, profile_page, list_page)
        write_atomically(argv[2], stats)
    except Exception as error:  # report every failure to the caller's log
        print(f"mdl_fetch failed: {error}", file=sys.stderr)
        return 1

    print(
        f"{stats['completed']} completed, {stats['watching']} watching, "
        f"{stats['plan_to_watch']} planned, {stats['episodes']} episodes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
