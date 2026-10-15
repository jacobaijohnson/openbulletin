# Desktop build runner (openbulletin-runner)

A dedicated GitHub Actions runner for this repo on Jacob's desktop, in
Docker. It shares nothing with the EV project's ev-runner: separate
image (`openbulletin-runner:local`), container (`openbulletin-runner`),
folder (`C:\\openbulletin-runner`), runner registration, labels
(`openbulletin`), and build caches. CPU cap 6 so it never starves the
EV runner (cap 10) on the 28-thread desktop. No Docker socket mount:
OpenBulletin CI runs plain cargo, so the runner does not need it.

## Recreate (5 minutes, e.g. when the access token expires)
1. Get a fresh token (registration token lasts 1 hour; a PAT lasts
   longer and is what the EV runner uses):
   `gh api -X POST repos/jacobaijohnson/openbulletin/actions/runners/registration-token --jq .token`
2. On the desktop, in `C:\\openbulletin-runner`:
   `docker build -t openbulletin-runner:local .`
3. `docker rm -f openbulletin-runner` then:
   `docker run -d --name openbulletin-runner --restart always --cpus=6 -e REPO_URL=https://github.com/jacobaijohnson/openbulletin -e ACCESS_TOKEN=<token> -e RUNNER_NAME=openbulletin-desktop -e LABELS=openbulletin openbulletin-runner:local`
4. Verify: `gh api repos/jacobaijohnson/openbulletin/actions/runners`
