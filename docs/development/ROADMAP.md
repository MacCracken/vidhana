# Vidhana Development Roadmap

> Last updated: 2026-03-18

## Current State

Vidhana has 6 crates with MVP persistence fully wired:

| Crate | Status | Notes |
|-------|--------|-------|
| `vidhana-core` | MVP | Types, shared state, TOML config, validation (clamp ranges) |
| `vidhana-api` | MVP | REST CRUD + PATCH for all 7 categories, history endpoints, JSON errors |
| `vidhana-ui` | MVP | egui GUI with all 7 panels + History + About, auto-save on change |
| `vidhana-ai` | Scaffold | Keyword-based NL parsing, no LLM/hoosh integration |
| `vidhana-settings` | MVP | TOML persistence + SQLite audit trail, wired into all mutation paths |
| `vidhana-mcp` | MVP | 5 MCP tools with persistence and change recording |

Settings persist automatically via TOML on every mutation (API, MCP, GUI). All changes are recorded in SQLite audit history. No OS-level system backends yet — settings are stored but not applied to the OS.

---

## MVP — Minimum Viable Product

Goal: A working settings app that reads and writes real config, persists across restarts, and exposes a functional API.

### Core

- [x] Fix `toml_from_str` placeholder in `vidhana-core` — use `toml::from_str` instead of `serde_json`
- [x] Add settings validation (brightness 0-100, volume 0-100, scaling 0.5-3.0, etc.)
- [x] Add `Display` impl for `SettingsCategory`

### Persistence

- [x] Auto-save settings on every mutation (GUI, API, MCP)
- [x] Wire `SettingsStore::record_change()` into API update handlers
- [x] Wire `SettingsStore::record_change()` into MCP `set` handlers
- [x] Pass `SettingsStore` through shared state via `AppState` / `Arc<SettingsStore>`

### API

- [x] Return proper error responses (`ApiError` -> JSON error body with status codes)
- [x] Add `PATCH` support for partial updates (JSON merge on top of current state)
- [x] Add `/v1/settings/history` endpoint for recent changes
- [x] Add `/v1/settings/{category}/history` endpoint

### GUI

- [x] Wire GUI changes to persistence (auto-save on every frame with dirty flag)
- [x] Add auto-save via dirty tracking (saves at end of frame and on panel switch)
- [ ] Add status bar showing API connection state
- [x] Add settings history panel (view recent changes in grid)

### MCP

- [x] Add `initialize` and `notifications/initialized` support per MCP spec
- [x] Return structured errors for invalid arguments / unknown tools

### Testing

- [x] Integration tests for API (start server, hit endpoints, verify persistence)
- [x] Round-trip test: save via API -> load state -> verify consistency
- [x] PATCH validation test (clamp out-of-range, reject invalid enums)
- [x] MCP persistence and history recording tests

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

- [x] Add `/v1/nl` API endpoint that accepts natural language and returns structured intent
- [ ] Integrate with hoosh (8088) for LLM-powered NL parsing as upgrade path
- [x] Fallback to local keyword parser when hoosh is unavailable (current default)
- [ ] Add NL input bar to GUI (text field at top of settings)

### API Improvements

- [ ] API versioning strategy (v1 stable contract)
- [ ] Rate limiting
- [x] Request/response logging middleware (tower-http TraceLayer)
- [ ] OpenAPI/Swagger spec generation
- [ ] WebSocket endpoint for real-time settings change notifications

### GUI Improvements

- [ ] Theming: apply selected theme to the settings app itself
- [ ] Toast notifications on successful/failed changes
- [ ] Search/filter across all settings panels
- [ ] Keyboard navigation and shortcuts
- [ ] Responsive layout for different window sizes

### MCP Improvements

- [x] MCP protocol compliance: initialize, capabilities, tools/list, tools/call
- [x] Add `vidhana_history` tool for querying change history (6 tools total)
- [ ] Add `vidhana_nl` tool for natural language settings via MCP
- [ ] SSE transport option (in addition to stdin/stdout)

### Quality

- [x] Structured error types in API (`ApiError` with JSON responses, proper status codes)
- [ ] Graceful degradation when system backends are unavailable
- [x] CI pipeline: `cargo test`, `cargo clippy`, `cargo fmt --check` (all pass clean)
- [x] Minimum test coverage for all public APIs (99 tests across workspace)
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
