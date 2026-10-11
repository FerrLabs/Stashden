# Upgrading

Stashden does not have a separate migration step. The server applies the database migrations it
carries when it starts, and refuses to serve until they have run.

## The routine

1. Take a backup, the database first and the blobs after it, as in
   [backup-and-restore.md](backup-and-restore.md). Migrations only move forward. There is no down
   migration, and the way back from a release that did something you did not want is the backup.
2. Stop the old server, then start the new one. Run a single instance: the local blob store is a
   directory, and two servers sweeping and counting the same one corrupt each other's reference
   counts. With the chart, `Recreate` does the stop-then-start for you.
3. Read the server's log for the first start. A migration that fails stops the start, and the
   database is left as it was before that one migration, since each runs in its own transaction.
4. Run `stashden-api check` once it is up. It is quick without `--verify-content` and costs nothing to
   leave in a post-upgrade routine.

Pin the image to an exact version in production rather than `latest`. The chart pins it through
`appVersion`, so upgrading the chart upgrades the image with it.

Downgrading is not supported. A server that finds a migration it does not know about refuses to
start, which is the safe outcome, and a database that a newer release has migrated has to go back
to the backup taken before it.

## Things a release changed for the people running it

These are the ones worth reading about before the upgrade. Everything else in a release is in its
notes on the releases page.

- **0.36.0: sessions can be revoked.** Session tokens now name a row in a `sessions` table, and a
  token without one is refused, so everybody signs in once more after the upgrade. Signing out, a
  password change, an administrator's reset and disabling an account end the sessions at once. The
  web app, and any script holding a token from `stashden login`, signs in again. App passwords are
  not sessions and are not affected.
- **0.32: the project was renamed from RoxyCloud.** Names that are kept on purpose for an instance
  that predates it, such as the `roxycloud` database and user in the compose file and the chart's
  `nameOverride`, are listed in the README.

## When something is wrong afterwards

- The server does not start and says a migration failed: restore the database from the backup,
  report the migration and the error, and stay on the previous release.
- Sign-in works and files are missing from a listing: this is data, not the upgrade. Run
  `stashden-api check --verify-content` and compare against the backup.
- A client cannot reach `/dav` or cannot upload anything above a few megabytes: it is the proxy, not
  the release. [reverse-proxy-and-tls.md](reverse-proxy-and-tls.md) lists what it has to allow.
