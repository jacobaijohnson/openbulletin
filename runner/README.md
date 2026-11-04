# Desktop build runner (openbulletin-runner)

A dedicated GitHub Actions runner for this repo on Jacob's desktop, in
Docker. It shares nothing with the EV project's ev-runner: separate
image (`openbulletin-runner:local`), container (`openbulletin-runner`),
folder (`C:\\openbulletin-runner`), runner registration, labels
(`openbulletin`), and build caches. CPU cap 6 so it never starves the
EV runner (cap 10) on the 28-thread desktop. No Docker socket mount:
OpenBulletin CI runs plain cargo, so the runner does not need it.

## Recreate — durable shape (since 2026-11-04; read this before any recreate)

Two earlier failure modes are designed out; do not recreate with the
old shape (registration token + no volume + updates on):

1. **Self-update hazard (incidents 2026-11-03, twice):** the runner
   self-updates by replacing its own binaries, so a Docker/host
   restart mid-update leaves `/actions-runner/bin` deleted and the
   container restart-looping with exit 127. Fixed by
   `DISABLE_AUTO_UPDATE=true`, which the entrypoint turns into
   `config.sh --disableupdate`. Consequence: the runner stays at
   the image's bundled version (2.338.0 as of 2026-11-04) until
   the image is deliberately rebuilt. Never re-enable updates
   without a plan for the restart window.
2. **Token-expiry hazard (incident 2026-11-04):** with reusage
   disabled, every container start re-registers, and a
   registration token lasts 1 hour — so the first restart after
   expiry loops with 404 on registration. Fixed by runner
   reusage: `CONFIGURED_ACTIONS_RUNNER_FILES_DIR` plus a named
   volume mounted there plus `DISABLE_AUTOMATIC_DEREGISTRATION=true`
   (required by the entrypoint when reusage is on). Config then
   persists across restarts AND recreates (the volume survives
   `docker rm`), restarts skip re-registration entirely ("already
   been configured"), and no token is needed after the first
   registration. Caveats, both observed: after a restart the
   runner takes ~2 minutes to reconnect (old-session expiry,
   self-healing retry loop — jobs queue, nothing is lost); and
   stopping the container leaves its GitHub entry offline until
   it reconnects under the same name/id.

Recreate steps (no image rebuild unless the Dockerfile changed —
`FROM myoung34/github-runner:latest` floats, so a rebuild can
silently bump the bundled runner version; reuse the existing
image for restarts/recreates and rebuild deliberately when a
version bump is wanted):

1. Mint a registration token ONLY if the first registration is
   needed (fresh volume, or volume + GitHub entry both wiped):
   `gh api -X POST repos/jacobaijohnson/openbulletin/actions/runners/registration-token --jq .token`
   (1 hour; run step 3 inside it). Otherwise no token is needed.
2. `docker rm -f openbulletin-runner` (the volume is NOT removed;
   never `docker rm -v` this container unless the GitHub entry
   is being wiped too).
3. `docker volume create openbulletin-runner-config` (no-op if
   it exists) then:
   `docker run -d --name openbulletin-runner --restart always --cpus=6 -v openbulletin-runner-config:/actions-runner-config -e REPO_URL=https://github.com/jacobaijohnson/openbulletin -e RUNNER_TOKEN=<token, first registration only> -e RUNNER_NAME=openbulletin-desktop -e LABELS=openbulletin -e DISABLE_AUTO_UPDATE=true -e DISABLE_AUTOMATIC_DEREGISTRATION=true -e CONFIGURED_ACTIONS_RUNNER_FILES_DIR=/actions-runner-config openbulletin-runner:local`
4. Verify: container log shows "Listening for Jobs"; after a
   `docker stop` + `docker start`, the log shows "already been
   configured" (no fresh registration); `gh api repos/jacobaijohnson/openbulletin/actions/runners` shows
   openbulletin-desktop online.

PAT alternative (not currently in use for this runner): the EV
runner's shape — `ACCESS_TOKEN` with the shared classic PAT,
which the entrypoint exchanges itself — re-registers on every
start and never expires on the 1-hour clock. Swapping to it
needs the PAT value through Jacob; it is not required while
reusage holds.
