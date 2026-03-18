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
            snap.muted = Some(out.contains("[MUTED]"));
            if let Some(vol_str) = out.split_whitespace().nth(1)
                && let Ok(vol) = vol_str.parse::<f32>()
            {
                snap.master_volume = Some((vol * 100.0).round() as u8);
            }
        } else if self.has_pactl {
            if let Some(out) = run_cmd_stdout("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]) {
                for word in out.split_whitespace() {
                    if let Some(pct) = word.strip_suffix('%')
                        && let Ok(v) = pct.parse::<u8>()
                    {
                        snap.master_volume = Some(v.min(100));
                        break;
                    }
                }
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
}
