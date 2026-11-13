"""Regression coverage for the retained upstream-name compatibility patch."""

import unittest

from verify import is_block_compatibility_patch


class CompatibilityPatchTests(unittest.TestCase):
    def test_reviewed_pin(self):
        self.assertTrue(is_block_compatibility_patch("crates-io", "block", {
            "git": "https://github.com/pierreaubert/sotf-3rdparties",
            "rev": "5a4634af0916b5f53effa5ef6340efd90af32285",
        }))

    def test_other_sources_and_revisions_require_review(self):
        pin = {
            "git": "https://github.com/pierreaubert/sotf-3rdparties",
            "rev": "5a4634af0916b5f53effa5ef6340efd90af32285",
        }
        for source, package, dependency in [
            ("other-registry", "block", pin),
            ("crates-io", "objc", pin),
            ("crates-io", "block", {**pin, "rev": "main"}),
            ("crates-io", "block", {**pin, "git": "https://example.org/fork"}),
            ("crates-io", "block", {**pin, "package": "sotf-block"}),
            ("crates-io", "block", {"version": "0.1.6"}),
            ("crates-io", "block", "0.1.6"),
        ]:
            with self.subTest(source=source, package=package, dependency=dependency):
                self.assertFalse(is_block_compatibility_patch(source, package, dependency))


if __name__ == "__main__":
    unittest.main()
