# Security Policy

## Supported versions

Until the first stable release, security fixes are made on the latest `main` branch and included in the next tagged release.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting for this repository. Do not open a public issue for a vulnerability that could expose users to command execution, privilege escalation, unsafe power behaviour, or local data disclosure.

Include the affected version or commit, Ubuntu version, session type, reproduction steps, and expected impact. You should receive an acknowledgement within seven days.

This project never requires `sudo` at runtime and should not write persistent system power settings. Treat any change to those properties as security-sensitive.
