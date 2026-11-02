# Beta release notifications

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
