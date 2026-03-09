# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in AmanClaw, please report it responsibly.

**Do NOT open a public issue.**

Instead, email: security@amanclaw.dev (or create a private security advisory on GitHub)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will respond within 48 hours and aim to release a fix within 7 days for critical issues.

## Scope

- Core engine and pipeline
- WASM plugin sandbox (escape vulnerabilities are critical)
- Authentication and authorization
- Input sanitization and injection detection
- Channel adapter security
- API endpoints

## Security Features

AmanClaw includes several security measures:
- WASM sandboxing for untrusted plugins (Wasmtime)
- OWASP Agentic Top 10 rule sets
- Input injection detection and sanitization
- Rate limiting per user
- Non-root Docker container with dropped capabilities
- Read-only filesystem in Docker
- Domain allowlists for plugin network access
