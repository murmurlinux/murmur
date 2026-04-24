# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Murmur, please report it responsibly.

**Do NOT open a public issue.** Instead:

1. Use GitHub's [private vulnerability reporting](https://github.com/murmurlinux/murmur/security/advisories/new)
2. Or email [security@murmurlinux.com](mailto:security@murmurlinux.com)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## Response Timeline

- **Acknowledgement:** within 48 hours
- **Initial assessment:** within 7 days
- **Fix or mitigation:** as soon as possible, targeting 30 days

## Scope

This policy covers:
- The Murmur desktop application (`murmurlinux/murmur`)
- The murmurlinux.com website (`murmurlinux/murmur-web`)
- Text injection via xdotool (command injection vectors)
- Audio capture and processing pipeline
- Model download and verification

## Out of Scope

- Third-party dependencies (report to the upstream project)
- Social engineering attacks
- Denial of service attacks

## Recognition

We appreciate responsible disclosure and will credit researchers in the changelog (unless you prefer anonymity).

## Recovery Procedures

These are the standing procedures Murmur maintainers follow if a credential or signing key is compromised.

### If a release-signing key is compromised

Release artefacts (the `.deb` and `.AppImage`) are signed with the Tauri updater signing key. If that key leaks or is suspected to have leaked:

1. Immediately revoke the associated `secrets.TAURI_SIGNING_PRIVATE_KEY` on `murmurlinux/murmur`.
2. Generate a new Tauri signing keypair locally; rotate the public key in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
3. Publish a new patch release using the new key. Existing installs will not accept updates signed with the new key until they have the new public key; communicate this via the changelog, the website, and a banner in the app if possible.
4. File a GitHub security advisory explaining the rotation and the steps users should take.

### If the APT signing GPG key is compromised

The APT repository (`murmurlinux.github.io/apt`) is signed with a repo-level GPG key held as `secrets.APT_SIGNING_KEY`. If that leaks:

1. Revoke the secret.
2. Generate a new GPG keypair; publish the new public key to `murmurlinux/apt/gpg.key`.
3. Rebuild and re-sign every `.deb` currently in the repository with the new key.
4. Users will see a "signature not valid" error on their next `apt update` until they re-import the key; document the one-liner in the changelog and on the download page.

### If a GitHub secret is exposed in a public repo

Secrets are scanned by TruffleHog on every push (see `.github/workflows/secret-scan.yml`) and by GitHub secret scanning with push protection. If either catches a leaked secret, or if one is discovered retroactively:

1. Rotate the leaked credential at the source (the service that issued it).
2. Update the corresponding GitHub secret on the affected repo.
3. If the leaked content is still reachable in git history, open a security advisory, decide whether history rewrite is warranted based on sensitivity, and document in the advisory.
4. Run `gh api repos/murmurlinux/murmur/secret-scanning/alerts` to confirm the alert was triaged.

### If CI infrastructure is compromised

A compromise of a third-party GitHub Action used in `release.yml` (Tauri action, APT deploy action, TruffleHog) could inject malicious code into signed artefacts. Mitigation:

- All third-party actions in the release path are pinned to a specific SHA (not a mutable `@main` or `@v1` tag). SHAs are listed in the header comment of each workflow file. Before bumping, verify the new SHA against the upstream project's changelog and release signatures.
- `release.yml` jobs run inside the `production` GitHub Environment, which requires a reviewer approval before the signing-key-holding steps execute.

If a compromise is suspected:

1. Revoke the signing keys (Tauri + APT GPG) as above.
2. Do not merge any pending release PRs until the upstream incident is understood and the action pin has been updated to a clean SHA.
3. Audit the last N releases for tampering (verify Tauri signatures against archived public keys).

### General incident response

For any security incident touching Murmur:

1. Open a GitHub security advisory (do not discuss in public issues).
2. Capture timestamps, affected versions, and blast radius.
3. Notify affected users via the changelog and, for severe issues, a website banner and an email to registered Pro users.
4. File a post-incident review in the private tracker with the root cause, timeline, and a list of prevention changes. Track the prevention changes to completion.
