# Releases

Release-plz maintains a release PR with independent versions and changelogs for
`systemd` and `libsystemd-sys`. Maintainers merge that PR when ready to release.
Release PRs are not merged automatically.

The initial versions are explicitly prepared as `systemd 0.11.0` for the journal
API changes and `libsystemd-sys 0.9.5` for the new FFI declarations.

After the merged commit passes the full `build` workflow on master, release-plz
publishes the crates in dependency order and creates GitHub releases. Tags keep
the existing `systemd-<version>` and `libsystemd-sys-<version>` formats.

The publish job checks out the exact commit tested by CI. Publishing is restricted
to successful master push runs from this repository and to merged release PRs
(`release_always = false`).

## Trusted publishing setup

Register a GitHub Actions trusted publisher in the crates.io settings for **both**
`systemd` and `libsystemd-sys`, using these values:

| Field | Value |
| --- | --- |
| GitHub owner | `codyps` |
| Repository | `rust-systemd` |
| Workflow filename | `release-plz.yml` |
| GitHub environment | `release` |

The GitHub environment `release` permits deployments from master. No crates.io
API token is supplied to the workflow. Release-plz exchanges the publish job's
OIDC identity for temporary credentials directly; a separate crates.io auth
action is not needed. The legacy `CARGO_REGISTRY_TOKEN` secret is unused.

See the [crates.io trusted publishing documentation](https://crates.io/docs/trusted-publishing).

## Release PR checks

The release PR job uses the repository's built-in GitHub token. After opening or
updating the PR, it synchronizes both README examples and the minimum
`libsystemd-sys` dependency, then explicitly dispatches `build.yml` on the final
release branch commit. This runs the required CI without needing a GitHub App,
personal access token, or manual workflow approval for the bot's PR event.

The minimum sys-crate dependency follows its workspace version so consumers cannot
resolve an older sys crate that lacks newly added FFI declarations.
Each crate has its own README and version examples, allowing high-level-only
documentation changes without triggering a sys-crate release.

## Reviewing and retrying releases

Review the versions, changelogs, and API-check results in the release PR. When
changing versions by hand, also run:

```sh
python3 .github/scripts/sync_release.py
cargo update --workspace
```

Dispatch the `Release-plz` workflow on master to refresh the release PR manually.
To retry a failed publication, rerun its publish job from the Actions page; it
retains the original tested commit. Configure trusted publishing for both crates
before merging the first release PR.
