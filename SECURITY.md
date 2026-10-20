# Security Policy

## Supported versions

Only the latest `0.11.x` release receives security fixes.

| Version  | Supported |
| -------- | --------- |
| 0.11.x   | yes       |
| < 0.11   | no        |

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Report privately through GitHub's private vulnerability reporting:
[open a security advisory](https://github.com/eRgo35/lyra/security/advisories/new)
(Security tab → *Report a vulnerability* on the
[eRgo35/lyra](https://github.com/eRgo35/lyra) repository).

Include the affected version, a description of the problem, and steps to reproduce it.
You can expect an acknowledgement as soon as the maintainer is able to review the report;
this is a small, volunteer-run project, so there is no guaranteed response time.

## Known advisories

Dependency advisories that are known and not yet resolved are tracked in the
[changelog](CHANGELOG.md) (see the `Security` sections).

## Deployment notes

Your Discord bot token is the only secret Lyra needs. Keep it in `.env` or the process
environment, never commit it, and regenerate it in the
[Developer Portal](https://discord.com/developers/applications) if it leaks.
