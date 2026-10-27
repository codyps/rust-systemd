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

After opening or updating the release PR, the job synchronizes both README examples
and the minimum `libsystemd-sys` dependency. A GitHub App installation token allows
the PR and subsequent pushes to trigger normal `pull_request` CI automatically.
The same token is used for checkout, release-plz, and metadata pushes. It is scoped
to this repository with Contents and Pull requests write permissions, and revoked
when the job finishes. Publishing uses the built-in GitHub token and crates.io OIDC.

Configure the App once:

1. [Create a private GitHub App](https://github.com/settings/apps/new?name=codyps-rust-systemd-releases&url=https%3A%2F%2Fgithub.com%2Fcodyps%2Frust-systemd&public=false&webhook_active=false&contents=write&pull_requests=write).
   Grant **Contents: Read and write** and **Pull requests: Read and write**.
   Leave webhooks disabled and installation limited to this account.
2. Install it on `codyps/rust-systemd` only. Generate a private key in its settings.
3. Store the downloaded PEM as the repository Actions secret
   `RELEASE_PLZ_APP_PRIVATE_KEY`, then set the repository Actions variable
   `RELEASE_PLZ_APP_CLIENT_ID` to the App's **Client ID**. For example:

   ```sh
   gh secret set RELEASE_PLZ_APP_PRIVATE_KEY --repo codyps/rust-systemd < /path/to/private-key.pem
   gh variable set RELEASE_PLZ_APP_CLIENT_ID --repo codyps/rust-systemd --body 'YOUR_APP_CLIENT_ID'
   ```

4. Dispatch `Release-plz` on master to refresh the release PR. Wait for its CI,
   then merge the release PR when ready.

Until the Client ID variable is configured, the job falls back to the built-in
GitHub token. In that mode, a maintainer must select **Approve workflows to run**
on the bot's PR; updates from the bot can require approval again.
Manually dispatched CI does not satisfy required PR checks; see
[GitHub's status check guidance](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks#checks-from-some-workflow-jobs-are-not-evaluated).

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
