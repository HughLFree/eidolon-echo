# Release Checklist

Use this checklist before cutting a public build of Eidolon-Echo.

## Core Runtime

- [ ] `cargo tauri dev` starts successfully from `apps/desktop/src-tauri`
- [ ] desktop shell windows appear and remain interactive
- [ ] backend sidecar starts and `GET /api/health` responds

## AI Flow

- [ ] provider `api_key` can be configured in Settings
- [ ] `default` mode can send a message and receive a reply
- [ ] `roleplay` mode can send a message and persist history
- [ ] invalid `api_key` shows a clear user-facing error

## Desktop Behavior

- [ ] tray `Quit` closes both `eidolon-echo-shell` and `eidolon-echo-backend`
- [ ] Settings -> `其他` -> `清除本地数据` completes and backend restarts
- [ ] history panel, bubble window and input window still open/close correctly after data reset

## Packaging

- [ ] `cargo tauri build` succeeds on the release machine
- [ ] packaged app launches on a clean macOS environment
- [ ] packaged app can save provider settings and send at least one successful reply

## Uninstall / Data Handling

- [ ] README uninstall steps match the current app name and identifier
- [ ] local data path is documented clearly
- [ ] plaintext local `api_key` storage is disclosed in README / SECURITY docs

## Release Notes

- [ ] changelog entry matches the actual shipped behavior
- [ ] README startup steps, support boundary and troubleshooting are up to date
- [ ] SECURITY.md is present and private reporting guidance is still valid
