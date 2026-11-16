# UJANI Public AI updater feed

The desktop updater reads the signed Tauri manifest from the repository's dedicated `updates` branch:

`https://raw.githubusercontent.com/asifshadab33/UJANI-PUBLIC-AI-/updates/latest.json`

This avoids GitHub's `/releases/latest/download` endpoint, which does not select draft or prerelease releases. UJANI currently publishes pre-alpha releases as prereleases, so that endpoint cannot reliably discover them.

The release workflow publishes the feed only after the release has passed its signed-artifact verification, evidence capture and production-release approval. It downloads the `latest.json` asset from that approved release, validates the manifest structure and semantic version, and refuses to replace a newer feed with an older or equal version.

The feed contains signed artifact metadata and release URLs; the updater still verifies the artifact signature against the public key injected into the signed production build. The feed itself is not a substitute for Tauri artifact signature verification.

If feed publication fails, the already-published release remains available through its release page, but automatic update discovery may remain on the previous version until the workflow is repaired and rerun. The release checklist must verify that the feed's version and platform artifact URLs match the approved release.

Do not publish a manifest from an unapproved draft release, and do not put signing private keys or passwords in this branch.
