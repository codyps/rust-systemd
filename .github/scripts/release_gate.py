"""Recognize a merged release-plz PR before enabling publication."""

import json
import os
import subprocess


def is_release_pr(pr: dict, repository: str, commit: str) -> bool:
    return (
        bool(pr.get("merged_at"))
        and pr.get("merge_commit_sha") == commit
        and pr.get("base", {}).get("ref") == "master"
        and pr.get("head", {}).get("repo", {}).get("full_name") == repository
        and pr.get("head", {}).get("ref", "").startswith("release-plz-")
        and "This PR was generated with [release-plz]" in (pr.get("body") or "")
    )


def main() -> None:
    repository = os.environ["GITHUB_REPOSITORY"]
    commit = os.environ["GITHUB_SHA"]
    prs = json.loads(
        subprocess.check_output(
            ["gh", "api", f"repos/{repository}/commits/{commit}/pulls"],
            text=True,
        )
    )
    if not isinstance(prs, list):
        raise ValueError("Expected a list of associated pull requests")
    allowed = any(is_release_pr(pr, repository, commit) for pr in prs)
    print(f"is_release={str(allowed).lower()}")


if __name__ == "__main__":
    main()
