# Vidhana — AGNOS System Settings

> Sanskrit: विधान (regulation, constitution, arrangement)

[![License](https://img.shields.io/badge/license-GPLv3-blue)](LICENSE)
[![Status](https://img.shields.io/badge/status-development-yellow)]()

**Vidhana** is the system settings application for [AGNOS](https://github.com/MacCracken/agnosticos).
Unified interface for display, audio, network, privacy, locale, power, and accessibility
with natural language control and MCP tool integration.

## Features

- **Settings GUI** — egui-based desktop application with tabbed interface
- **Natural language** — "make the screen brighter", "turn off bluetooth", "switch to dark mode"
- **HTTP API** — REST endpoints on port 8099 for programmatic access
- **MCP tools** — 5 tools for agent-driven settings management
- **Settings history** — SQLite audit trail of all changes
- **AGNOS integration** — connects to daimon (8090) and hoosh (8088)
- **Headless mode** — API-only for server/headless deployments

## Architecture

```
vidhana
├── vidhana-core       — Settings types, shared state
├── vidhana-api        — REST API (axum) on port 8099
├── vidhana-ui         — egui/eframe desktop GUI
├── vidhana-ai         — Natural language settings parsing
├── vidhana-settings   — TOML + SQLite persistence, change history
└── vidhana-mcp        — MCP tool server (5 tools)
```

## Settings Categories

| Category | Controls |
|----------|----------|
| Display | Brightness, theme, scaling, night light, high contrast |
| Audio | Volume, mute, output/input devices |
| Network | WiFi, Bluetooth, firewall, VPN, DNS, proxy |
| Privacy | Screen lock, telemetry, camera, microphone, agent approval |
| Locale | Language, region, timezone, keyboard layout, clock format |
| Power | Power profile, suspend, lid close, display timeout |
| Accessibility | Large text, reduce motion, screen reader, sticky keys, cursor size |

## Usage

```bash
# Launch GUI
vidhana --gui

# Headless mode (API only)
vidhana --headless --port 8099

# MCP server (stdin/stdout JSON-RPC)
vidhana --mcp
```

## MCP Tools

- `vidhana_display` — Get/set display settings
- `vidhana_audio` — Get/set audio settings
- `vidhana_network` — Get/set network settings
- `vidhana_privacy` — Get/set privacy settings
- `vidhana_system` — Get/set power, locale, accessibility

## License

GPL-3.0
