import copy
import unittest

from release_gate import is_release_pr


class ReleaseGateTests(unittest.TestCase):
    def setUp(self):
        self.repository = "codyps/rust-systemd"
        self.commit = "tested-merge-commit"
        self.pr = {
            "merged_at": "2026-10-27T00:00:00Z",
            "merge_commit_sha": self.commit,
            "base": {"ref": "main"},
            "head": {
                "ref": "release-plz-2026-10-27T04-46-49Z",
                "repo": {"full_name": self.repository},
            },
            "body": "This PR was generated with [release-plz](https://github.com/release-plz/release-plz/).",
        }

    def test_accepts_merged_generated_release_pr(self):
        self.assertTrue(is_release_pr(self.pr, self.repository, self.commit))

    def test_rejects_setup_branch_sharing_release_prefix(self):
        self.pr["head"]["ref"] = "release-plz-app-token"
        self.pr["body"] = "Add GitHub App support to the release workflow."
        self.assertFalse(is_release_pr(self.pr, self.repository, self.commit))

    def test_rejects_unmerged_or_unrelated_pr(self):
        for field, value in (("merged_at", None), ("merge_commit_sha", "other")):
            with self.subTest(field=field):
                pr = copy.deepcopy(self.pr)
                pr[field] = value
                self.assertFalse(is_release_pr(pr, self.repository, self.commit))

    def test_rejects_other_base_repository_or_branch(self):
        for path, value in (
            (("base", "ref"), "other"),
            (("base", "ref"), "master"),
            (("head", "repo", "full_name"), "other/repository"),
            (("head", "ref"), "feature-branch"),
        ):
            with self.subTest(path=path):
                pr = copy.deepcopy(self.pr)
                node = pr
                for key in path[:-1]:
                    node = node[key]
                node[path[-1]] = value
                self.assertFalse(is_release_pr(pr, self.repository, self.commit))

    def test_rejects_missing_release_metadata(self):
        self.assertFalse(is_release_pr({}, self.repository, self.commit))
        self.pr["body"] = None
        self.assertFalse(is_release_pr(self.pr, self.repository, self.commit))


if __name__ == "__main__":
    unittest.main()
