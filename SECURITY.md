# Security Policy

## Supported Versions

We actively support the following versions of rez-core with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability in rez-core, please report it to us privately.

### How to Report

1. **Email**: Send details to hal.long@outlook.com
2. **GitHub Security Advisory**: Use GitHub's private vulnerability reporting feature
3. **Include**: 
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt within 48 hours
- **Initial Assessment**: We will provide an initial assessment within 5 business days
- **Updates**: We will keep you informed of our progress
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days

### Security Measures

This project implements several security measures:

- **Dependency Scanning**: Automated dependency vulnerability scanning via Dependabot
- **Code Analysis**: Static code analysis using CodeQL
- **Supply Chain Security**: OSSF Scorecard monitoring
- **Audit**: Regular Rust security audits using `cargo audit`
- **Hardened CI/CD**: Security-hardened GitHub Actions workflows

### Responsible Disclosure

We follow responsible disclosure practices:

1. We will work with you to understand and validate the vulnerability
2. We will develop and test a fix
3. We will coordinate the release of the fix
4. We will publicly acknowledge your contribution (unless you prefer to remain anonymous)

### Security Best Practices

When using rez-core:

- Keep dependencies up to date
- Use the latest stable version
- Follow secure coding practices
- Validate all inputs
- Use appropriate access controls

## Contact

For security-related questions or concerns, contact:
- Email: hal.long@outlook.com
- GitHub: @loonghao
