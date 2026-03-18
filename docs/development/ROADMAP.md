# Vidhana Development Roadmap

> Last updated: 2026-03-18

## Current State

Vidhana has 7 crates with full v1 architecture, 199 tests, ~80% line coverage:

| Crate | Status | Coverage | Notes |
|-------|--------|----------|-------|
| `vidhana-core` | v1 | 92% | Types, shared state, TOML config, validation |
| `vidhana-api` | v1 | 93% | REST CRUD + PATCH, history, NL endpoint, hoosh |
| `vidhana-ui` | v1 | 5% | egui GUI, 9 panels, auto-save via SettingsService |
| `vidhana-ai` | v1 | 87% | Keyword NL parser + async hoosh HTTP client |
| `vidhana-settings` | v1 | 100% | TOML persistence + SQLite audit trail |
| `vidhana-mcp` | v1 | 86% | 6 MCP tools + initialize handshake |
| `vidhana-backend` | v1 | 83% | SystemBackend trait, LinuxBackend, SettingsService mediator |

All mutations flow through `SettingsService`: validate -> apply to OS -> update state -> persist -> audit.

---

## Remaining v1 Items

### System Backends

- [ ] Display: theme integration (GTK/Qt theme switching, or AGNOS-specific)
- [ ] Display: night light via gammastep/wlsunset or similar
- [ ] Audio: enumerate and switch output/input devices
- [ ] Network: read real hostname, DNS from `/etc/resolv.conf` or systemd-resolved
- [ ] Network: firewall status via nftables/iptables
- [ ] Power: configure suspend/lid-close via logind D-Bus
- [ ] Locale: keyboard layout via XKB / sway input config
- [ ] Accessibility: integrate with AT-SPI / orca for screen reader
- [ ] Privacy: screen lock via swaylock/swayidle or loginctl

### Natural Language

- [ ] Add NL input bar to GUI (text field at top of settings)

### API

- [ ] API versioning strategy (v1 stable contract)
- [ ] Rate limiting
- [ ] OpenAPI/Swagger spec generation
- [ ] WebSocket endpoint for real-time settings change notifications

### GUI

- [ ] Theming: apply selected theme to the settings app itself
- [ ] Toast notifications on successful/failed changes
- [ ] Search/filter across all settings panels
- [ ] Keyboard navigation and shortcuts
- [ ] Responsive layout for different window sizes
- [ ] Status bar showing API connection state

### MCP

- [ ] Add `vidhana_nl` tool for natural language settings via MCP
- [ ] SSE transport option (in addition to stdin/stdout)

### Quality

- [ ] Graceful degradation when system backends are unavailable
- [ ] man page / `--help` improvements
- [ ] UI end-to-end testing via headless egui or screenshot-based regression

---

## Post-v1 — Polish & Integration

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
