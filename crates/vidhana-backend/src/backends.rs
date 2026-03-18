//! System backend trait and implementations.

use std::process::Command;
use vidhana_core::*;

/// Errors from system backend operations.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("command failed: {cmd} — {stderr}")]
    CommandFailed { cmd: String, stderr: String },

    #[error("device unavailable: {0}")]
    DeviceUnavailable(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

/// A snapshot of OS state read from system backends.
/// Each field is `Option` because individual reads may fail gracefully.
#[derive(Debug, Default)]
pub struct SystemSnapshot {
    pub brightness: Option<u8>,
    pub master_volume: Option<u8>,
    pub muted: Option<bool>,
    pub wifi_enabled: Option<bool>,
    pub bluetooth_enabled: Option<bool>,
    pub power_profile: Option<PowerProfile>,
    pub timezone: Option<String>,
}

/// Trait for applying settings to the operating system.
pub trait SystemBackend: Send + Sync {
    /// Read current OS state. Individual fields may be None if unreadable.
    fn read_system_state(&self) -> SystemSnapshot;

    fn apply_display(&self, settings: &DisplaySettings) -> Result<(), BackendError>;
    fn apply_audio(&self, settings: &AudioSettings) -> Result<(), BackendError>;
    fn apply_network(&self, settings: &NetworkSettings) -> Result<(), BackendError>;
    fn apply_power(&self, settings: &PowerSettings) -> Result<(), BackendError>;
    fn apply_locale(&self, settings: &LocaleSettings) -> Result<(), BackendError>;
    fn apply_privacy(&self, settings: &PrivacySettings) -> Result<(), BackendError>;
    fn apply_accessibility(&self, settings: &AccessibilitySettings) -> Result<(), BackendError>;
}

// ---------------------------------------------------------------------------
// Linux backend — uses CLI tools
// ---------------------------------------------------------------------------

/// Linux system backend that shells out to standard CLI tools.
pub struct LinuxBackend {
    pub has_brightnessctl: bool,
    pub has_wpctl: bool,
    pub has_pactl: bool,
    pub has_nmcli: bool,
    pub has_bluetoothctl: bool,
    pub has_powerprofilesctl: bool,
    pub has_timedatectl: bool,
    pub has_loginctl: bool,
}

impl LinuxBackend {
    /// Probe the system for available CLI tools.
    pub fn detect() -> Self {
        Self {
            has_brightnessctl: has_cmd("brightnessctl"),
            has_wpctl: has_cmd("wpctl"),
            has_pactl: has_cmd("pactl"),
            has_nmcli: has_cmd("nmcli"),
            has_bluetoothctl: has_cmd("bluetoothctl"),
            has_powerprofilesctl: has_cmd("powerprofilesctl"),
            has_timedatectl: has_cmd("timedatectl"),
            has_loginctl: has_cmd("loginctl"),
        }
    }

    /// Log which tools were found.
    pub fn log_capabilities(&self) {
        let tools = [
            ("brightnessctl", self.has_brightnessctl),
            ("wpctl", self.has_wpctl),
            ("pactl", self.has_pactl),
            ("nmcli", self.has_nmcli),
            ("bluetoothctl", self.has_bluetoothctl),
            ("powerprofilesctl", self.has_powerprofilesctl),
            ("timedatectl", self.has_timedatectl),
            ("loginctl", self.has_loginctl),
        ];
        for (name, available) in tools {
            if available {
                tracing::info!("Backend: {name} available");
            } else {
                tracing::debug!("Backend: {name} not found");
            }
        }
    }
}

impl SystemBackend for LinuxBackend {
    fn read_system_state(&self) -> SystemSnapshot {
        let mut snap = SystemSnapshot::default();

        // Brightness
        if self.has_brightnessctl
            && let Some(pct) = run_cmd_stdout("brightnessctl", &["-m", "info"])
                .and_then(|out| parse_brightnessctl(&out))
        {
            snap.brightness = Some(pct);
        }

        // Volume via wpctl
        if self.has_wpctl
            && let Some(out) = run_cmd_stdout("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])
        {
            let (vol, muted) = parse_wpctl_volume(&out);
            snap.master_volume = vol;
            snap.muted = muted;
        } else if self.has_pactl {
            if let Some(out) = run_cmd_stdout("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) {
                snap.master_volume = parse_pactl_volume(&out);
            }
            if let Some(out) = run_cmd_stdout("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]) {
                snap.muted = Some(out.contains("yes"));
            }
        }

        // WiFi
        if self.has_nmcli
            && let Some(out) = run_cmd_stdout("nmcli", &["radio", "wifi"])
        {
            snap.wifi_enabled = Some(out.trim() == "enabled");
        }

        // Bluetooth
        if self.has_bluetoothctl
            && let Some(out) = run_cmd_stdout("bluetoothctl", &["show"])
        {
            snap.bluetooth_enabled = Some(out.contains("Powered: yes"));
        }

        // Power profile
        if self.has_powerprofilesctl
            && let Some(out) = run_cmd_stdout("powerprofilesctl", &["get"])
        {
            snap.power_profile = match out.trim() {
                "performance" => Some(PowerProfile::Performance),
                "balanced" => Some(PowerProfile::Balanced),
                "power-saver" => Some(PowerProfile::PowerSaver),
                _ => None,
            };
        }

        // Timezone
        if self.has_timedatectl
            && let Some(out) =
                run_cmd_stdout("timedatectl", &["show", "--property=Timezone", "--value"])
        {
            let tz = out.trim().to_string();
            if !tz.is_empty() {
                snap.timezone = Some(tz);
            }
        }

        snap
    }

    fn apply_display(&self, settings: &DisplaySettings) -> Result<(), BackendError> {
        if self.has_brightnessctl {
            run_cmd(
                "brightnessctl",
                &["set", &format!("{}%", settings.brightness)],
            )?;
        }
        Ok(())
    }

    fn apply_audio(&self, settings: &AudioSettings) -> Result<(), BackendError> {
        if self.has_wpctl {
            let vol = format!("{:.2}", settings.master_volume as f32 / 100.0);
            run_cmd("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &vol])?;
            let mute_flag = if settings.muted { "1" } else { "0" };
            run_cmd("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", mute_flag])?;
        } else if self.has_pactl {
            run_cmd(
                "pactl",
                &[
                    "set-sink-volume",
                    "@DEFAULT_SINK@",
                    &format!("{}%", settings.master_volume),
                ],
            )?;
            let mute_val = if settings.muted { "1" } else { "0" };
            run_cmd("pactl", &["set-sink-mute", "@DEFAULT_SINK@", mute_val])?;
        }
        Ok(())
    }

    fn apply_network(&self, settings: &NetworkSettings) -> Result<(), BackendError> {
        if self.has_nmcli {
            let wifi_state = if settings.wifi_enabled { "on" } else { "off" };
            run_cmd("nmcli", &["radio", "wifi", wifi_state])?;
        }
        if self.has_bluetoothctl {
            let bt_state = if settings.bluetooth_enabled {
                "on"
            } else {
                "off"
            };
            run_cmd("bluetoothctl", &["power", bt_state])?;
        }
        Ok(())
    }

    fn apply_power(&self, settings: &PowerSettings) -> Result<(), BackendError> {
        if self.has_powerprofilesctl {
            let profile = match settings.power_profile {
                PowerProfile::Performance => "performance",
                PowerProfile::Balanced => "balanced",
                PowerProfile::PowerSaver => "power-saver",
            };
            run_cmd("powerprofilesctl", &["set", profile])?;
        }
        Ok(())
    }

    fn apply_locale(&self, settings: &LocaleSettings) -> Result<(), BackendError> {
        if self.has_timedatectl && !settings.timezone.is_empty() {
            run_cmd("timedatectl", &["set-timezone", &settings.timezone])?;
        }
        Ok(())
    }

    fn apply_privacy(&self, _settings: &PrivacySettings) -> Result<(), BackendError> {
        // Privacy settings (screen lock timeout, camera/mic access) are
        // typically managed by the desktop environment or compositor.
        // We store the preference; actual enforcement is compositor-specific.
        Ok(())
    }

    fn apply_accessibility(&self, _settings: &AccessibilitySettings) -> Result<(), BackendError> {
        // Accessibility settings (large text, screen reader) are typically
        // managed by AT-SPI / desktop environment integration.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Noop backend (for tests and sandboxed environments)
// ---------------------------------------------------------------------------

/// A no-op backend that does nothing. Used in tests and containers.
pub struct NoopBackend;

impl SystemBackend for NoopBackend {
    fn read_system_state(&self) -> SystemSnapshot {
        SystemSnapshot::default()
    }

    fn apply_display(&self, _: &DisplaySettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_audio(&self, _: &AudioSettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_network(&self, _: &NetworkSettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_power(&self, _: &PowerSettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_locale(&self, _: &LocaleSettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_privacy(&self, _: &PrivacySettings) -> Result<(), BackendError> {
        Ok(())
    }
    fn apply_accessibility(&self, _: &AccessibilitySettings) -> Result<(), BackendError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn has_cmd(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), BackendError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|_| BackendError::CommandNotFound(cmd.to_string()))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("Permission denied") || stderr.contains("Not authorized") {
            Err(BackendError::PermissionDenied(format!("{cmd}: {stderr}")))
        } else {
            Err(BackendError::CommandFailed {
                cmd: format!("{cmd} {}", args.join(" ")),
                stderr,
            })
        }
    }
}

fn run_cmd_stdout(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
}

/// Parse wpctl get-volume output to get volume and mute state.
/// Format: "Volume: 0.75" or "Volume: 0.75 [MUTED]"
fn parse_wpctl_volume(output: &str) -> (Option<u8>, Option<bool>) {
    let muted = Some(output.contains("[MUTED]"));
    let volume = output
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<f32>().ok())
        .map(|v| (v * 100.0).round() as u8);
    (volume, muted)
}

/// Parse pactl get-sink-volume output for percentage.
fn parse_pactl_volume(output: &str) -> Option<u8> {
    output.split_whitespace().find_map(|word| {
        word.strip_suffix('%')
            .and_then(|pct| pct.parse::<u8>().ok())
            .map(|v| v.min(100))
    })
}

/// Parse brightnessctl -m output to get percentage.
/// Format: "device,class,current,max,percentage"
fn parse_brightnessctl(output: &str) -> Option<u8> {
    // Machine-readable output: "acpi_video0,backlight,80,100,80%"
    for line in output.lines() {
        if let Some(pct_str) = line.split(',').next_back()
            && let Some(num) = pct_str.strip_suffix('%')
            && let Ok(v) = num.parse::<u8>()
        {
            return Some(v.min(100));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_backend_read() {
        let backend = NoopBackend;
        let snap = backend.read_system_state();
        assert!(snap.brightness.is_none());
        assert!(snap.master_volume.is_none());
    }

    #[test]
    fn test_noop_backend_apply() {
        let backend = NoopBackend;
        assert!(backend.apply_display(&DisplaySettings::default()).is_ok());
        assert!(backend.apply_audio(&AudioSettings::default()).is_ok());
        assert!(backend.apply_network(&NetworkSettings::default()).is_ok());
        assert!(backend.apply_power(&PowerSettings::default()).is_ok());
        assert!(backend.apply_locale(&LocaleSettings::default()).is_ok());
        assert!(backend.apply_privacy(&PrivacySettings::default()).is_ok());
        assert!(
            backend
                .apply_accessibility(&AccessibilitySettings::default())
                .is_ok()
        );
    }

    #[test]
    fn test_linux_backend_detect() {
        // Just ensure detection doesn't panic
        let backend = LinuxBackend::detect();
        // At minimum, `which` itself exists on Linux
        let _ = backend.has_brightnessctl;
    }

    #[test]
    fn test_parse_brightnessctl_output() {
        assert_eq!(
            parse_brightnessctl("acpi_video0,backlight,80,100,80%"),
            Some(80)
        );
        assert_eq!(
            parse_brightnessctl("intel_backlight,backlight,255,255,100%"),
            Some(100)
        );
        assert_eq!(parse_brightnessctl(""), None);
        assert_eq!(parse_brightnessctl("garbage"), None);
    }

    #[test]
    fn test_has_cmd() {
        // `which` is always available on Linux
        assert!(has_cmd("ls"));
        assert!(!has_cmd("definitely_not_a_real_command_xyz"));
    }

    #[test]
    fn test_system_snapshot_default() {
        let snap = SystemSnapshot::default();
        assert!(snap.brightness.is_none());
        assert!(snap.master_volume.is_none());
        assert!(snap.wifi_enabled.is_none());
        assert!(snap.bluetooth_enabled.is_none());
        assert!(snap.power_profile.is_none());
        assert!(snap.timezone.is_none());
    }

    #[test]
    fn test_backend_error_display() {
        let err = BackendError::CommandNotFound("brightnessctl".to_string());
        assert!(err.to_string().contains("brightnessctl"));

        let err = BackendError::CommandFailed {
            cmd: "nmcli radio wifi on".to_string(),
            stderr: "not authorized".to_string(),
        };
        assert!(err.to_string().contains("nmcli"));
    }

    #[test]
    fn test_backend_error_variants() {
        let err = BackendError::DeviceUnavailable("backlight".to_string());
        assert!(err.to_string().contains("backlight"));

        let err = BackendError::PermissionDenied("timedatectl".to_string());
        assert!(err.to_string().contains("timedatectl"));
    }

    // --- run_cmd / run_cmd_stdout tests ---

    #[test]
    fn test_run_cmd_success() {
        // `true` always succeeds
        assert!(run_cmd("true", &[]).is_ok());
    }

    #[test]
    fn test_run_cmd_failure() {
        // `false` always fails with exit code 1
        let err = run_cmd("false", &[]).unwrap_err();
        assert!(matches!(err, BackendError::CommandFailed { .. }));
    }

    #[test]
    fn test_run_cmd_not_found() {
        let err = run_cmd("definitely_not_a_real_cmd_xyz", &[]).unwrap_err();
        assert!(matches!(err, BackendError::CommandNotFound(_)));
    }

    #[test]
    fn test_run_cmd_stdout_success() {
        let out = run_cmd_stdout("echo", &["hello"]);
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));
    }

    #[test]
    fn test_run_cmd_stdout_failure() {
        let out = run_cmd_stdout("false", &[]);
        assert!(out.is_none());
    }

    #[test]
    fn test_run_cmd_stdout_not_found() {
        let out = run_cmd_stdout("definitely_not_a_real_cmd_xyz", &[]);
        assert!(out.is_none());
    }

    // --- LinuxBackend with no capabilities ---

    fn no_tools_backend() -> LinuxBackend {
        LinuxBackend {
            has_brightnessctl: false,
            has_wpctl: false,
            has_pactl: false,
            has_nmcli: false,
            has_bluetoothctl: false,
            has_powerprofilesctl: false,
            has_timedatectl: false,
            has_loginctl: false,
        }
    }

    #[test]
    fn test_linux_backend_no_tools_read_state() {
        let backend = no_tools_backend();
        let snap = backend.read_system_state();
        // All None when no tools available
        assert!(snap.brightness.is_none());
        assert!(snap.master_volume.is_none());
        assert!(snap.muted.is_none());
        assert!(snap.wifi_enabled.is_none());
        assert!(snap.bluetooth_enabled.is_none());
        assert!(snap.power_profile.is_none());
        assert!(snap.timezone.is_none());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_display() {
        let backend = no_tools_backend();
        assert!(backend.apply_display(&DisplaySettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_audio() {
        let backend = no_tools_backend();
        assert!(backend.apply_audio(&AudioSettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_network() {
        let backend = no_tools_backend();
        assert!(backend.apply_network(&NetworkSettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_power() {
        let backend = no_tools_backend();
        assert!(backend.apply_power(&PowerSettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_locale() {
        let backend = no_tools_backend();
        assert!(backend.apply_locale(&LocaleSettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_privacy() {
        let backend = no_tools_backend();
        assert!(backend.apply_privacy(&PrivacySettings::default()).is_ok());
    }

    #[test]
    fn test_linux_backend_no_tools_apply_accessibility() {
        let backend = no_tools_backend();
        assert!(
            backend
                .apply_accessibility(&AccessibilitySettings::default())
                .is_ok()
        );
    }

    #[test]
    fn test_linux_backend_apply_locale_empty_timezone() {
        // Even with timedatectl "available", empty timezone should be a no-op
        let backend = LinuxBackend {
            has_timedatectl: true,
            ..no_tools_backend()
        };
        let settings = LocaleSettings {
            timezone: String::new(),
            ..LocaleSettings::default()
        };
        assert!(backend.apply_locale(&settings).is_ok());
    }

    #[test]
    fn test_log_capabilities_no_panic() {
        let backend = no_tools_backend();
        backend.log_capabilities();
        // Also test with some tools "available"
        let detected = LinuxBackend::detect();
        detected.log_capabilities();
    }

    // --- LinuxBackend read_system_state with real tools (best-effort) ---

    #[test]
    fn test_linux_backend_real_read_state() {
        // Uses detect() to find real tools — results vary by environment
        // but should never panic
        let backend = LinuxBackend::detect();
        let snap = backend.read_system_state();
        // Just verify it returns without error
        // On CI: most fields will be None (no desktop tools)
        // On desktop: some fields will have values
        let _ = snap.brightness;
        let _ = snap.master_volume;
        let _ = snap.muted;
        let _ = snap.wifi_enabled;
        let _ = snap.bluetooth_enabled;
        let _ = snap.power_profile;
        let _ = snap.timezone;
    }

    #[test]
    fn test_parse_brightnessctl_multiline() {
        // Some systems output multiple devices
        let output = "device1,class,50,100,50%\ndevice2,class,75,100,75%";
        // Should return the first match
        assert_eq!(parse_brightnessctl(output), Some(50));
    }

    #[test]
    fn test_parse_brightnessctl_over_100() {
        // Should clamp to 100
        assert_eq!(parse_brightnessctl("device,class,255,255,150%"), Some(100));
    }

    // --- wpctl / pactl parsing ---

    #[test]
    fn test_parse_wpctl_volume_normal() {
        let (vol, muted) = parse_wpctl_volume("Volume: 0.75");
        assert_eq!(vol, Some(75));
        assert_eq!(muted, Some(false));
    }

    #[test]
    fn test_parse_wpctl_volume_muted() {
        let (vol, muted) = parse_wpctl_volume("Volume: 0.50 [MUTED]");
        assert_eq!(vol, Some(50));
        assert_eq!(muted, Some(true));
    }

    #[test]
    fn test_parse_wpctl_volume_zero() {
        let (vol, muted) = parse_wpctl_volume("Volume: 0.00");
        assert_eq!(vol, Some(0));
        assert_eq!(muted, Some(false));
    }

    #[test]
    fn test_parse_wpctl_volume_garbage() {
        let (vol, muted) = parse_wpctl_volume("garbage");
        assert!(vol.is_none());
        assert_eq!(muted, Some(false));
    }

    #[test]
    fn test_parse_pactl_volume() {
        assert_eq!(
            parse_pactl_volume("Volume: front-left: 65536 / 100% / 0.00 dB"),
            Some(100)
        );
    }

    #[test]
    fn test_parse_pactl_volume_partial() {
        assert_eq!(
            parse_pactl_volume("Volume: front-left: 42000 / 64% / -11.70 dB"),
            Some(64)
        );
    }

    #[test]
    fn test_parse_pactl_volume_no_match() {
        assert_eq!(parse_pactl_volume("no percentage here"), None);
    }

    // --- LinuxBackend with real tools (conditional) ---

    #[test]
    fn test_linux_backend_read_timezone_if_available() {
        if !has_cmd("timedatectl") {
            return; // skip on systems without timedatectl
        }
        let backend = LinuxBackend {
            has_timedatectl: true,
            ..no_tools_backend()
        };
        let snap = backend.read_system_state();
        // timedatectl should return a timezone
        assert!(snap.timezone.is_some());
        assert!(!snap.timezone.unwrap().is_empty());
    }

    /// Backend with all flags true, even if tools don't exist.
    /// This exercises the code paths inside the `if self.has_*` branches.
    fn all_tools_backend() -> LinuxBackend {
        LinuxBackend {
            has_brightnessctl: true,
            has_wpctl: true,
            has_pactl: true,
            has_nmcli: true,
            has_bluetoothctl: true,
            has_powerprofilesctl: true,
            has_timedatectl: true,
            has_loginctl: true,
        }
    }

    #[test]
    fn test_linux_backend_all_flags_read_state() {
        // Forces all branches to execute; commands may fail but shouldn't panic
        let backend = all_tools_backend();
        let snap = backend.read_system_state();
        // Results depend on whether tools are actually installed
        let _ = snap;
    }

    #[test]
    fn test_linux_backend_all_flags_apply_display() {
        let backend = all_tools_backend();
        // May fail (tool not installed or no backlight device), but should not panic
        let _ = backend.apply_display(&DisplaySettings::default());
    }

    #[test]
    fn test_linux_backend_all_flags_apply_audio() {
        let backend = all_tools_backend();
        let _ = backend.apply_audio(&AudioSettings::default());
    }

    #[test]
    fn test_linux_backend_all_flags_apply_network() {
        let backend = all_tools_backend();
        let _ = backend.apply_network(&NetworkSettings::default());
    }

    #[test]
    fn test_linux_backend_all_flags_apply_power() {
        let backend = all_tools_backend();
        let _ = backend.apply_power(&PowerSettings::default());
    }

    #[test]
    fn test_linux_backend_all_flags_apply_locale() {
        let backend = all_tools_backend();
        let mut locale = LocaleSettings::default();
        locale.timezone = "UTC".to_string();
        let _ = backend.apply_locale(&locale);
    }

    #[test]
    fn test_linux_backend_all_flags_apply_privacy() {
        let backend = all_tools_backend();
        let _ = backend.apply_privacy(&PrivacySettings::default());
    }

    #[test]
    fn test_linux_backend_all_flags_apply_accessibility() {
        let backend = all_tools_backend();
        let _ = backend.apply_accessibility(&AccessibilitySettings::default());
    }

    #[test]
    fn test_linux_backend_read_bluetooth_if_available() {
        if !has_cmd("bluetoothctl") {
            return; // skip
        }
        let backend = LinuxBackend {
            has_bluetoothctl: true,
            ..no_tools_backend()
        };
        let snap = backend.read_system_state();
        // bluetoothctl show should return a powered state (true or false)
        assert!(snap.bluetooth_enabled.is_some());
    }
}
