import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import mdl_fetch  # noqa: E402

FIXTURES = Path(__file__).resolve().parent / "fixtures"


def read_fixture(name: str) -> str:
    return (FIXTURES / name).read_text(encoding="utf-8")


class ParseProfileTest(unittest.TestCase):
    def test_reads_totals_and_avatar(self):
        profile = mdl_fetch.parse_profile(read_fixture("profile.html"), "ExampleUser")
        self.assertEqual(profile["episodes"], 728)
        self.assertEqual(profile["shows"], 42)
        self.assertEqual(profile["movies"], 2)
        self.assertEqual(profile["show_time"], "28d 10h 28m")
        self.assertEqual(profile["show_minutes"], 28 * 24 * 60 + 10 * 60 + 28)
        self.assertEqual(profile["movie_time"], "3h 57m")
        self.assertEqual(profile["movie_minutes"], 3 * 60 + 57)
        self.assertEqual(profile["avatar_url"], "https://i.mydramalist.com/AAAAAA_1c.jpg")

    def test_profile_without_stats_gives_zeroes(self):
        profile = mdl_fetch.parse_profile(read_fixture("profile_empty.html"), "Hilary")
        self.assertEqual(profile["episodes"], 0)
        self.assertEqual(profile["shows"], 0)
        self.assertEqual(profile["movies"], 0)
        self.assertIsNone(profile["show_time"])
        self.assertEqual(profile["show_minutes"], 0)

    def test_non_profile_page_is_rejected(self):
        with self.assertRaises(mdl_fetch.ParseError):
            mdl_fetch.parse_profile("<title>Just a moment...</title>", "ExampleUser")


FULL_LIST = ["dramalist.html", "dramalist_page2.html"]


class ParseDramalistTest(unittest.TestCase):
    def test_counts_each_status_across_pages(self):
        pages = [read_fixture(name) for name in FULL_LIST]
        counts = mdl_fetch.parse_dramalist(pages, "ExampleUser")
        self.assertEqual(counts["watching"], 1)
        self.assertEqual(counts["completed"], 43)
        self.assertEqual(counts["plan_to_watch"], 80)
        self.assertEqual(counts["on_hold"], 0)
        self.assertEqual(counts["dropped"], 0)

    def test_first_page_holds_only_the_first_hundred_rows(self):
        # Regression: counting the first page alone reported 56 planned, not 80.
        rows = mdl_fetch.parse_list_rows(read_fixture("dramalist.html"))
        self.assertEqual(len(rows), mdl_fetch.LIST_PAGE_SIZE)

    def test_overlapping_pages_count_rows_once(self):
        first = read_fixture("dramalist.html")
        counts = mdl_fetch.parse_dramalist([first, first], "ExampleUser")
        self.assertEqual(counts["plan_to_watch"], 56)

    def test_mean_score_skips_unrated_entries(self):
        pages = [read_fixture(name) for name in FULL_LIST]
        summary = mdl_fetch.parse_dramalist(pages, "ExampleUser")
        self.assertEqual(summary["rated"], 43)
        self.assertEqual(summary["mean_score"], 8.98)

    def test_top_rated_keeps_every_tied_title(self):
        pages = [read_fixture(name) for name in FULL_LIST]
        top = mdl_fetch.parse_dramalist(pages, "ExampleUser")["top_rated"]
        self.assertEqual(top["score"], 10.0)
        self.assertEqual(len(top["titles"]), 6)
        self.assertEqual(top["titles"][0], "Go Ahead")

    def test_countries_count_completed_titles_only(self):
        pages = [read_fixture(name) for name in FULL_LIST]
        countries = mdl_fetch.parse_dramalist(pages, "ExampleUser")["countries"]
        self.assertEqual(countries, {"South Korea": 32, "China": 11})
        self.assertEqual(list(countries)[0], "South Korea")

    def test_titles_are_unescaped(self):
        rows = mdl_fetch.parse_list_rows(read_fixture("dramalist.html"))
        titles = {entry["title"] for entry in rows.values()}
        self.assertIn("Lighter & Princess", titles)

    def test_empty_list_has_no_scores(self):
        summary = mdl_fetch.summarize_scores([])
        self.assertEqual(summary, {"mean_score": None, "rated": 0, "top_rated": None})

    def test_non_list_page_is_rejected(self):
        with self.assertRaises(mdl_fetch.ParseError):
            mdl_fetch.parse_dramalist(["<title>Just a moment...</title>"], "ExampleUser")

    def test_challenge_instead_of_later_page_is_rejected(self):
        pages = [read_fixture("dramalist.html"), "<title>Just a moment...</title>"]
        with self.assertRaises(mdl_fetch.ParseError):
            mdl_fetch.parse_dramalist(pages, "ExampleUser")


class FakeResponse:
    def __init__(self, text, status_code=200):
        self.text = text
        self.status_code = status_code


class FakeSession:
    def __init__(self, get_text, post_responses):
        self.get_text = get_text
        self.post_responses = list(post_responses)
        self.posted_pages = []

    def get(self, url, timeout):
        return FakeResponse(self.get_text)

    def post(self, url, json, timeout):
        self.posted_pages.append(json["page"])
        return self.post_responses.pop(0)


class FetchDramalistPagesTest(unittest.TestCase):
    def test_follows_pages_until_a_short_one(self):
        session = FakeSession(
            read_fixture("dramalist.html"),
            [FakeResponse(read_fixture("dramalist_page2.html"))],
        )
        pages = mdl_fetch.fetch_dramalist_pages(session, "ExampleUser")
        self.assertEqual(len(pages), 2)
        self.assertEqual(session.posted_pages, [2])

    def test_http_error_on_later_page_fails(self):
        session = FakeSession(read_fixture("dramalist.html"), [FakeResponse("", 403)])
        with self.assertRaises(RuntimeError):
            mdl_fetch.fetch_dramalist_pages(session, "ExampleUser")


class BuildStatsTest(unittest.TestCase):
    def test_combines_pages_into_stats(self):
        stats = mdl_fetch.build_stats(
            "ExampleUser",
            read_fixture("profile.html"),
            [read_fixture(name) for name in FULL_LIST],
        )
        self.assertEqual(stats["username"], "ExampleUser")
        self.assertEqual(stats["profile_url"], "https://mydramalist.com/profile/ExampleUser")
        self.assertEqual(stats["list_url"], "https://mydramalist.com/dramalist/ExampleUser")
        self.assertEqual(stats["completed"], 43)
        self.assertEqual(stats["episodes"], 728)
        self.assertIsInstance(stats["fetched_at"], int)
        json.dumps(stats)  # must be serializable


class UsernameValidationTest(unittest.TestCase):
    def test_rejects_path_characters(self):
        for bad in ["", "a/b", "../x", "a b", "a?b=1"]:
            with self.assertRaises(ValueError):
                mdl_fetch.validate_username(bad)

    def test_accepts_normal_username(self):
        self.assertEqual(mdl_fetch.validate_username("ExampleUser"), "ExampleUser")


if __name__ == "__main__":
    unittest.main()
