# Changelog

All notable changes to Vidhana will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [2026.3.18] - 2026-03-18

### Added — Initial Scaffold

- **vidhana-core**: Settings types (Display, Audio, Network, Privacy, Locale, Power, Accessibility), SharedState, SettingsCategory with NL aliases
- **vidhana-api**: REST API on port 8099 with GET/POST for all 7 categories, health endpoint, daimon client
- **vidhana-ui**: egui/eframe settings GUI with tabbed side navigation, sliders, combo boxes, checkboxes
- **vidhana-ai**: Natural language settings parser (brightness, theme, volume, mute, wifi, bluetooth, firewall, night light, power profile, screen reader, large text, timezone)
- **vidhana-settings**: TOML config persistence, SQLite change history with audit trail
- **vidhana-mcp**: 5 MCP tools (vidhana_display, vidhana_audio, vidhana_network, vidhana_privacy, vidhana_system)
- **CLI**: `--gui`, `--headless`, `--mcp`, `--port`, `--bind`, `--data-dir` flags
- **CI/CD**: GitHub Actions with check, lint, security audit, test, build, release (amd64 + arm64)
