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
        profile = mdl_fetch.parse_profile(read_fixture("profile.html"), "ImSe4n")
        self.assertEqual(profile["episodes"], 728)
        self.assertEqual(profile["shows"], 42)
        self.assertEqual(profile["movies"], 2)
        self.assertEqual(profile["show_time"], "28d 10h 28m")
        self.assertEqual(profile["avatar_url"], "https://i.mydramalist.com/VXykQy_1c.jpg")

    def test_profile_without_stats_gives_zeroes(self):
        profile = mdl_fetch.parse_profile(read_fixture("profile_empty.html"), "Hilary")
        self.assertEqual(profile["episodes"], 0)
        self.assertEqual(profile["shows"], 0)
        self.assertEqual(profile["movies"], 0)
        self.assertIsNone(profile["show_time"])

    def test_non_profile_page_is_rejected(self):
        with self.assertRaises(mdl_fetch.ParseError):
            mdl_fetch.parse_profile("<title>Just a moment...</title>", "ImSe4n")


class ParseDramalistTest(unittest.TestCase):
    def test_counts_each_status(self):
        counts = mdl_fetch.parse_dramalist(read_fixture("dramalist.html"), "ImSe4n")
        self.assertEqual(counts["watching"], 1)
        self.assertEqual(counts["completed"], 43)
        self.assertEqual(counts["plan_to_watch"], 56)
        self.assertEqual(counts["on_hold"], 0)
        self.assertEqual(counts["dropped"], 0)

    def test_non_list_page_is_rejected(self):
        with self.assertRaises(mdl_fetch.ParseError):
            mdl_fetch.parse_dramalist("<title>Just a moment...</title>", "ImSe4n")


class BuildStatsTest(unittest.TestCase):
    def test_combines_pages_into_stats(self):
        stats = mdl_fetch.build_stats(
            "ImSe4n", read_fixture("profile.html"), read_fixture("dramalist.html")
        )
        self.assertEqual(stats["username"], "ImSe4n")
        self.assertEqual(stats["profile_url"], "https://mydramalist.com/profile/ImSe4n")
        self.assertEqual(stats["list_url"], "https://mydramalist.com/dramalist/ImSe4n")
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
        self.assertEqual(mdl_fetch.validate_username("ImSe4n"), "ImSe4n")


if __name__ == "__main__":
    unittest.main()
