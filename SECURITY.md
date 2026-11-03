# Handling credentials

Keep secrets in local ignored environment files or a secret manager. `.env.example` must contain placeholders only. Generate JWT keys using `keys/README.md`. Client credentials and runtime settings must remain untracked.

Install Gitleaks and enable the local commit guard:

```sh
brew install gitleaks
git config core.hooksPath .githooks
```

Before publishing, scan all history with `gitleaks git --log-opts=--all --redact`. GitHub secret scanning and push protection supplement this check and may not recognize custom credentials.

If credentials leak, revoke or rotate them with the provider immediately. Rotate signing keys and invalidate sessions; migrate encrypted OAuth data before retiring an encryption key. Rewriting history does not revoke credentials or erase existing clones and cached commits.
