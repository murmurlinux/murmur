# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Murmur, please report it responsibly.

**Do NOT open a public issue.** Instead:

1. Use GitHub's [private vulnerability reporting](https://github.com/murmurlinux/murmur/security/advisories/new)
2. Or email [dev@murmurlinux.com](mailto:dev@murmurlinux.com)

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
