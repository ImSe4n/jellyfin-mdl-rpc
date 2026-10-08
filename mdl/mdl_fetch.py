"""Fetch public MyDramaList profile stats and write them to a JSON file.

Read-only: it loads the public profile and drama list pages (including the
extra list pages the site's infinite scroll loads), the same pages anyone can
open in a browser. MDL sits behind Cloudflare, which rejects plain
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
# The drama list page only renders its first LIST_PAGE_SIZE rows; the rest load
# page by page, like the site's own infinite scroll. MAX_LIST_PAGES caps the loop.
LIST_PAGE_SIZE = 100
MAX_LIST_PAGES = 50
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

# One drama list table row: its id, then the cells each card needs.
ROW_PATTERN = re.compile(r'<tr[^>]*id="ml(\d+)"(.*?)</tr>', flags=re.S)
CELL_PATTERNS = {
    "title": re.compile(r'<a title="([^"]*)"'),
    "status": re.compile(r'<td class="msv2-i-status">([^<]*)</td>'),
    "country": re.compile(r'<td class="msv2-i-country">([^<]*)</td>'),
    "score": re.compile(r'<span class="score">([^<]*)</span>'),
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


def _duration_minutes(text: str) -> int:
    """Turn '28d 10h 28m' into total minutes."""
    units = {"d": 24 * 60, "h": 60, "m": 1}
    return sum(int(n) * units[u] for n, u in re.findall(r"(\d+)\s*([dhm])", text))


def parse_profile(page: str, username: str) -> dict:
    _require_title(page, f"{username}'s Profile")

    profile = {
        "episodes": 0,
        "shows": 0,
        "movies": 0,
        "show_time": None,
        "movie_time": None,
        "show_minutes": 0,
        "movie_minutes": 0,
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
            profile["show_minutes"] = _duration_minutes(profile["show_time"])
        elif movies:
            profile["movies"] = int(movies.group(1))
            profile["movie_time"] = _duration_text(duration)
            profile["movie_minutes"] = _duration_minutes(profile["movie_time"])

    avatar = re.search(
        r'<img class="img-responsive mdl-rounded" src="(https://[^"]+)" alt="'
        + re.escape(username)
        + '"',
        page,
    )
    if avatar:
        profile["avatar_url"] = avatar.group(1)

    return profile


def _cell(row: str, name: str) -> str:
    match = CELL_PATTERNS[name].search(row)
    return html.unescape(match.group(1)).strip() if match else ""


def parse_row(row: str) -> dict:
    try:
        score = float(_cell(row, "score"))
    except ValueError:
        score = 0.0
    return {
        "title": _cell(row, "title"),
        "status": _cell(row, "status"),
        "country": _cell(row, "country"),
        # MDL shows 0.0 for unrated entries.
        "score": score if score > 0 else None,
    }


def parse_list_rows(page: str) -> dict:
    """Map each list row's id to its entry, so overlapping pages count once."""
    if "msv2-table" not in page:
        raise ParseError("drama list page has no list table")
    return {row_id: parse_row(row) for row_id, row in ROW_PATTERN.findall(page)}


def count_statuses(entries: list) -> dict:
    counts = {key: 0 for key in set(STATUS_KEYS.values())}
    for entry in entries:
        key = STATUS_KEYS.get(entry["status"])
        if key:
            counts[key] += 1
    return counts


def summarize_scores(entries: list) -> dict:
    """Mean of the rated entries, and every title tied for the highest score."""
    rated = [entry for entry in entries if entry["score"] is not None]
    if not rated:
        return {"mean_score": None, "rated": 0, "top_rated": None}
    best = max(entry["score"] for entry in rated)
    return {
        "mean_score": round(sum(entry["score"] for entry in rated) / len(rated), 2),
        "rated": len(rated),
        "top_rated": {
            "score": best,
            "titles": [entry["title"] for entry in rated if entry["score"] == best],
        },
    }


def count_countries(entries: list) -> dict:
    """Completed titles per country, most first."""
    counts = {}
    for entry in entries:
        if entry["status"] == "Completed" and entry["country"]:
            counts[entry["country"]] = counts.get(entry["country"], 0) + 1
    return dict(sorted(counts.items(), key=lambda item: (-item[1], item[0])))


def parse_dramalist(pages: list, username: str) -> dict:
    """Summarize the full list page followed by any extra list pages."""
    _require_title(pages[0], f"{username}'s Drama List")

    rows = {}
    for page in pages:
        rows.update(parse_list_rows(page))
    entries = list(rows.values())
    return {
        **count_statuses(entries),
        **summarize_scores(entries),
        "countries": count_countries(entries),
    }


def build_stats(username: str, profile_page: str, list_pages: list) -> dict:
    return {
        "username": username,
        "profile_url": f"{BASE_URL}/profile/{username}",
        "list_url": f"{BASE_URL}/dramalist/{username}",
        "fetched_at": int(time.time()),
        **parse_profile(profile_page, username),
        **parse_dramalist(list_pages, username),
    }


def _check_status(response, method: str, url: str) -> str:
    if response.status_code != 200:
        raise RuntimeError(f"{method} {url} returned HTTP {response.status_code}")
    return response.text


def fetch_page(session, url: str) -> str:
    response = session.get(url, timeout=REQUEST_TIMEOUT_SECONDS)
    return _check_status(response, "GET", url)


def fetch_dramalist_pages(session, username: str) -> list:
    """The full list page, then each further page until one comes back short."""
    url = f"{BASE_URL}/dramalist/{username}"
    pages = [fetch_page(session, url)]
    page_number = 1
    while len(parse_list_rows(pages[-1])) >= LIST_PAGE_SIZE:
        page_number += 1
        if page_number > MAX_LIST_PAGES:
            raise RuntimeError(f"drama list still going after {MAX_LIST_PAGES} pages")
        response = session.post(
            url, json={"page": page_number}, timeout=REQUEST_TIMEOUT_SECONDS
        )
        pages.append(_check_status(response, "POST", url))
    return pages


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
        from curl_cffi import requests  # imported here so tests don't need it

        username = validate_username(argv[1])
        with requests.Session(impersonate="chrome") as session:
            profile_page = fetch_page(session, f"{BASE_URL}/profile/{username}")
            list_pages = fetch_dramalist_pages(session, username)
        stats = build_stats(username, profile_page, list_pages)
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
