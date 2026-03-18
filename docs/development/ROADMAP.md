# Vidhana Development Roadmap

> Last updated: 2026-03-18

## Current State

Vidhana has a complete scaffold with 6 crates:

| Crate | Status | Notes |
|-------|--------|-------|
| `vidhana-core` | Scaffold | Types, shared state, config loading (TOML parsing is a placeholder) |
| `vidhana-api` | Scaffold | REST CRUD for all 7 categories, health endpoint, DaimonClient stub |
| `vidhana-ui` | Scaffold | egui GUI with all 7 panels + About, side nav |
| `vidhana-ai` | Scaffold | Keyword-based NL parsing, no LLM/hoosh integration |
| `vidhana-settings` | Scaffold | TOML persistence + SQLite audit trail |
| `vidhana-mcp` | Scaffold | 5 MCP tools (display, audio, network, privacy, system) |

All settings are in-memory only. No system backends exist. Changes made via GUI/API/MCP are not persisted automatically and do not affect the actual OS.

---

## MVP — Minimum Viable Product

Goal: A working settings app that reads and writes real config, persists across restarts, and exposes a functional API.

### Core

- [ ] Fix `toml_from_str` placeholder in `vidhana-core` — use `toml::from_str` instead of `serde_json`
- [ ] Add settings validation (brightness 0-100, volume 0-100, valid timezone strings, etc.)
- [ ] Add `Default` display for `SettingsCategory`

### Persistence

- [ ] Auto-save settings on every mutation (GUI, API, MCP)
- [ ] Wire `SettingsStore::record_change()` into API update handlers
- [ ] Wire `SettingsStore::record_change()` into MCP `set` handlers
- [ ] Pass `SettingsStore` through shared state (currently only loaded at startup in `main.rs`)

### API

- [ ] Return proper error responses (`ApiError` -> JSON error body with status codes)
- [ ] Add `PATCH` support for partial updates (current `POST` requires full struct replacement)
- [ ] Add `/v1/settings/history` endpoint for recent changes
- [ ] Add `/v1/settings/{category}/history` endpoint

### GUI

- [ ] Wire GUI changes to persistence (save on change)
- [ ] Add unsaved-changes indicator or auto-save confirmation
- [ ] Add status bar showing API connection state
- [ ] Add settings history panel (view recent changes)

### MCP

- [ ] Add `initialize` and `notifications` support per MCP spec
- [ ] Return structured errors for invalid arguments

### Testing

- [ ] Integration tests for API (start server, hit endpoints, verify persistence)
- [ ] Round-trip test: save via API -> load via GUI -> verify consistency

---

## v1 — First Release

Goal: Real system integration, natural language control, production-quality error handling.

### System Backends

- [ ] Display: read/write brightness via sysfs (`/sys/class/backlight/`)
- [ ] Display: theme integration (GTK/Qt theme switching, or AGNOS-specific)
- [ ] Display: night light via gammastep/wlsunset or similar
- [ ] Audio: PipeWire/PulseAudio volume control via `libpulse` or `wpctl`
- [ ] Audio: enumerate and switch output/input devices
- [ ] Network: WiFi toggle via NetworkManager D-Bus
- [ ] Network: Bluetooth toggle via bluez D-Bus
- [ ] Network: read real hostname, DNS from `/etc/resolv.conf` or systemd-resolved
- [ ] Network: firewall status via nftables/iptables
- [ ] Power: read/set power profile via `power-profiles-daemon` D-Bus
- [ ] Power: configure suspend/lid-close via logind D-Bus
- [ ] Locale: read/set timezone via `timedatectl` / systemd-timedated D-Bus
- [ ] Locale: keyboard layout via XKB / sway input config
- [ ] Accessibility: integrate with AT-SPI / orca for screen reader
- [ ] Privacy: screen lock via swaylock/swayidle or loginctl

### Natural Language

- [ ] Add `/v1/nl` API endpoint that accepts natural language and returns structured intent
- [ ] Integrate with hoosh (8088) for LLM-powered NL parsing as upgrade path
- [ ] Fallback to local keyword parser when hoosh is unavailable
- [ ] Add NL input bar to GUI (text field at top of settings)

### API Improvements

- [ ] API versioning strategy (v1 stable contract)
- [ ] Rate limiting
- [ ] Request/response logging middleware
- [ ] OpenAPI/Swagger spec generation
- [ ] WebSocket endpoint for real-time settings change notifications

### GUI Improvements

- [ ] Theming: apply selected theme to the settings app itself
- [ ] Toast notifications on successful/failed changes
- [ ] Search/filter across all settings panels
- [ ] Keyboard navigation and shortcuts
- [ ] Responsive layout for different window sizes

### MCP Improvements

- [ ] Full MCP protocol compliance (capabilities, resources, prompts)
- [ ] Add `vidhana_history` tool for querying change history
- [ ] Add `vidhana_nl` tool for natural language settings via MCP
- [ ] SSE transport option (in addition to stdin/stdout)

### Quality

- [ ] Structured error types across all crates (no `.unwrap()` in production paths)
- [ ] Graceful degradation when system backends are unavailable
- [ ] CI pipeline: `cargo test`, `cargo clippy`, `cargo fmt --check`
- [ ] Minimum test coverage for all public APIs
- [ ] man page / `--help` improvements

---

## Post-v1 — Polish & Integration

Goal: Deep AGNOS ecosystem integration, better UX, operational maturity.

### AGNOS Integration

- [ ] Daimon (8090) integration: show service status in About panel
- [ ] Daimon health monitoring: show system metrics in GUI
- [ ] Hoosh (8088) integration: conversational settings management
- [ ] Register vidhana MCP tools with AGNOS agent registry
- [ ] System notifications via AGNOS notification daemon

### Settings Management

- [ ] Settings profiles (e.g., "Work", "Home", "Presentation")
- [ ] Profile auto-switching based on context (network, time, location)
- [ ] Settings import/export (TOML file)
- [ ] Settings reset to defaults (per-category and global)
- [ ] Undo/redo for recent changes

### GUI Polish

- [ ] Custom icons for each settings category
- [ ] Animations and transitions (respecting reduce-motion preference)
- [ ] First-run wizard / onboarding flow
- [ ] System tray integration with quick-access controls
- [ ] Per-panel "Advanced" sections for power users

### Operational

- [ ] Structured logging (JSON output for log aggregation)
- [ ] Metrics export (Prometheus endpoint)
- [ ] Health check improvements (dependency status: daimon, hoosh, system backends)
- [ ] Systemd service file and socket activation
- [ ] Package builds (deb, rpm, pacman, flatpak)

---

## v2 — Advanced Features

Goal: Multi-user, multi-device, extensibility, and advanced display management.

### Multi-Display

- [ ] Enumerate connected displays (via DRM/KMS or wlr-output-management)
- [ ] Per-display brightness, scaling, refresh rate
- [ ] Display arrangement editor in GUI
- [ ] Display profiles (docked, undocked, projector)

### Multi-User

- [ ] Per-user settings storage (XDG base directories)
- [ ] System-wide vs user-level settings distinction
- [ ] Admin-locked settings (prevent users from changing certain values)
- [ ] Polkit integration for privileged operations

### Remote Management

- [ ] Remote settings API with authentication (token/mTLS)
- [ ] Fleet-wide settings deployment (push config to multiple machines)
- [ ] Settings sync across devices (via AGNOS sync service)
- [ ] Audit log export and compliance reporting

### Extensibility

- [ ] Plugin system for third-party settings panels
- [ ] Custom settings categories via plugin API
- [ ] Theme engine for GUI (custom color schemes, fonts)
- [ ] Scripting hooks (run commands on settings change)
- [ ] D-Bus interface for desktop environment integration

### Advanced AI

- [ ] Context-aware settings suggestions ("it's getting dark, enable night light?")
- [ ] Settings anomaly detection (unusual changes, potential misconfigurations)
- [ ] Natural language settings search ("where do I change the font size?")
- [ ] Voice control integration
- [ ] Learning user preferences over time (auto-adjust based on patterns)
