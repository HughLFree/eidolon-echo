# Homebrew Tap Skeleton

This directory contains a third-party Homebrew tap skeleton for distributing `Eidolon-Echo` as a macOS cask.

## What This Gives You

- `brew install --cask eidolon-echo`
- `brew uninstall --cask eidolon-echo`
- `brew uninstall --cask --zap eidolon-echo` for a more complete uninstall

The included `zap` intentionally removes the local application data directory:

- `~/Library/Application Support/io.github.hughlfree.eidolonecho`
- `~/Library/Caches/io.github.hughlfree.eidolonecho`
- `~/Library/Logs/io.github.hughlfree.eidolonecho`
- `~/Library/Preferences/io.github.hughlfree.eidolonecho.plist`

That means `--zap` removes local chats, provider settings, and stored API keys.

## Expected Release Asset

The cask assumes each GitHub release exposes this asset name:

- `Eidolon-Echo.dmg`

and that the DMG contains:

- `Eidolon-Echo.app`

If your release asset names differ, update `Casks/eidolon-echo.rb`.

## Recommended Repo Layout

Create a separate tap repository:

- `HughLFree/homebrew-eidolon-echo`

Then copy this file into that repository:

- `Casks/eidolon-echo.rb`

## User Commands

Install:

```bash
brew tap HughLFree/eidolon-echo
brew install --cask eidolon-echo
```

Uninstall app only:

```bash
brew uninstall --cask eidolon-echo
```

Uninstall app and local data:

```bash
brew uninstall --cask --zap eidolon-echo
```

## Important Notes

- This is a third-party tap. Homebrew does not support third-party taps the same way it supports official Homebrew repositories.
- The current cask uses `version :latest` and `sha256 :no_check` for simplicity. That is fine for an early self-maintained tap, but a later stable release flow should move to explicit versioned assets and checksums.
