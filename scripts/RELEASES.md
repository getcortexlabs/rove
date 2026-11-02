# macOS releases

## In-app updates

Rove uses [Sparkle 2](https://sparkle-project.org/documentation/) for signed
updates. The appcast lives at `https://rove.getcortexlabs.com/updates/appcast.xml`
and archives live in GitHub release assets. The feed is explicit: GitHub's
`latest` redirect excludes prereleases, so it is not used to select beta builds.
No beta token, email address or system profile is attached to update checks.

Local builds leave updates disabled. Distribution packaging requires a Rove
Ed25519 public key; it embeds the key, enables daily automatic checks and leaves
automatic downloads/install-on-quit off until the user chooses them in Settings.
Updates never cancel an analysis. An installation requested during active work
waits for another click in Settings or the Rove menu after the work finishes.

The [0.1.2 distribution record](release-checks/0.1.2.md) tracks the first signed
upgrade exercise and the remaining publication checks.

Before the first distribution using Sparkle:

1. The Rove-specific key is provisioned in the release machine's Keychain under
   the `rove` account. Resolve the Swift package and use its
   `bin/generate_keys --account rove -p` tool to confirm the public key matches
   `apps/macos/SparklePublicKey.txt`. Keep its protected backup outside the
   repository. Never regenerate the key for a release or reuse Lily's key.
   Losing this key requires a planned key migration.
2. Distribution packaging reads `apps/macos/SparklePublicKey.txt` by default.
   `ROVE_SPARKLE_PUBLIC_KEY` can override it for an isolated signing test.
   The private key is never put in the bundle or passed as a command argument.
   If packaging moves to CI, store the private key in its secret store and
   supply it through a restricted temporary file.
3. Increase `CFBundleVersion` (a positive integer) on every distribution and set
   `CFBundleShortVersionString` to the release version. Preserve the bundle ID
   `com.getcortexlabs.rove.preview`, which also owns existing beta preferences.
4. Build with the existing `CODESIGN_ID` and `NOTARY_PROFILE` credentials:

   ```sh
   scripts/package-macos.sh
   python3 scripts/make-appcast.py build/macos/Rove-VERSION.dmg \
     --key-file /secure/path/rove.key --notes /path/to/release-notes.txt
   ```

   Packaging embeds Sparkle and signs its nested helpers before the containing
   framework and app. The feed generator mounts the final DMG read-only, checks
   signatures, notarization and bundle identity, signs the final archive with
   Sparkle, then verifies the signature against the key in the shipped app.
   It writes `build/macos/appcast.xml`; it does not publish anything.
5. Test an upgrade between **two signed, notarized builds** with distinct build
   numbers on a separate app copy. Check manual/automatic discovery, skipping,
   an active analysis, installation, relaunch, retained history and beta access.
   Unit tests verify signatures and deferral but do not replace this release test.
6. Upload the exact DMG as `Rove-VERSION.dmg` on a draft `vVERSION` release.
   Publish the release, then copy the generated feed to
   `docs/public/updates/appcast.xml` in a reviewed PR and deploy the docs site.
   Keep old feed items. Never publish a feed pointing at an unavailable draft
   asset, or replace a DMG after signing it. Publishing also triggers the beta
   email job below; checking for updates does not send email.

The empty initial feed deliberately advertises no release. Existing 0.1.1
installations have no updater: testers need one manual DMG installation before
they can receive subsequent versions in the app. Neither a code merge nor the
empty feed activates updates for the old DMG. Release key provisioning and the
two-build installation test are required before the first live update.

If a release must be withdrawn, remove its item from the feed. To fix an already
installed build, publish a higher build number; do not offer a downgrade or edit
an archive in place. See [Sparkle's publishing guide](https://sparkle-project.org/documentation/publishing/).

## Beta release notifications

A published GitHub release (stable or prerelease) triggers
`.github/workflows/beta-release.yml`. It sends the release ID to the existing
Cortex Labs beta service. The service verifies the release and its macOS DMG,
snapshots active invitations and sends one email per recipient with download and
release-note links. Waiting-list entries alone are excluded.

Before merging/enabling the workflow, deploy the companion change in
[getcontexlabs-site](https://github.com/getcortexlabs/getcontexlabs-site/tree/main/src/server/rove):
apply `migrations/rove-release-mail.sql`, set a new `ROVE_RELEASE_SECRET` on Pages,
and deploy its Functions. Store the same secret in this repository's Actions
secrets. The backend uses the existing Resend credentials; those credentials and
the invitation list never enter this repository or its Actions runner.

Upload the DMG before publishing the release. Commits, tags, drafts and edits do
not send notices. The implementation does not backfill old releases. If publishing
from another workflow in the future, note that events created with `GITHUB_TOKEN`
do not normally trigger another workflow: explicitly dispatch this workflow using
an appropriate token, or call the notification client in the publication job.
See [GitHub's event documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#release).

The workflow logs counts only, retries transient failures and exits unsuccessfully
if the queue cannot finish. Rerun the job or use **Run workflow** with the same
numeric release ID to resume. Accepted deliveries are recorded and not resent.
After 23 hours, an uncertain provider outcome requires reconciliation in the
backend, before [Resend's 24-hour idempotency window](https://resend.com/docs/dashboard/emails/idempotency-keys)
expires. The backend README documents that procedure. Provider acceptance does
not establish inbox delivery; check the provider's delivery/bounce records.

Local validation sends no email:

```sh
node --test scripts/notify-beta.test.mjs
```

The backend tests use real SQLite transactions and a fake mail provider. Never
run the production client as a smoke test: it is a send operation for a published
release. This setup requires no change to the Rust engine or beta access tokens.
