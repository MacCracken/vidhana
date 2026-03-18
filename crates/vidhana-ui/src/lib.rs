//! Vidhana UI — egui-based system settings application
//!
//! Tabbed interface for managing all AGNOS system settings.

use std::sync::Arc;
use vidhana_core::*;
use vidhana_settings::{ChangeSource, SettingsChange, SettingsStore};

/// Active settings panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Display,
    Audio,
    Network,
    Privacy,
    Locale,
    Power,
    Accessibility,
    History,
    About,
}

impl Panel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Audio => "Audio",
            Self::Network => "Network",
            Self::Privacy => "Privacy",
            Self::Locale => "Language & Region",
            Self::Power => "Power",
            Self::Accessibility => "Accessibility",
            Self::History => "History",
            Self::About => "About",
        }
    }

    pub fn all() -> &'static [Panel] {
        &[
            Self::Display,
            Self::Audio,
            Self::Network,
            Self::Privacy,
            Self::Locale,
            Self::Power,
            Self::Accessibility,
            Self::History,
            Self::About,
        ]
    }
}

/// Main Vidhana application
pub struct VidhanaApp {
    state: SharedState,
    store: Arc<SettingsStore>,
    active_panel: Panel,
    dirty: bool,
}

impl VidhanaApp {
    pub fn new(state: SharedState, store: Arc<SettingsStore>) -> Self {
        Self {
            state,
            store,
            active_panel: Panel::Display,
            dirty: false,
        }
    }

    fn save_if_dirty(&mut self) {
        if self.dirty {
            let guard = self.state.read().unwrap();
            if let Err(e) = self.store.save_state(&guard) {
                tracing::error!("Failed to save settings: {e}");
            }
            self.dirty = false;
        }
    }

    fn record_change(&self, category: &str, key: &str, old: &str, new: &str) {
        let change = SettingsChange {
            timestamp: chrono::Utc::now(),
            category: category.to_string(),
            key: key.to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
            source: ChangeSource::Gui,
        };
        if let Err(e) = self.store.record_change(&change) {
            tracing::error!("Failed to record change: {e}");
        }
    }
}

impl eframe::App for VidhanaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Side navigation
        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui.heading("Vidhana");
                ui.label("System Settings");
                ui.separator();

                for panel in Panel::all() {
                    if ui
                        .selectable_label(self.active_panel == *panel, panel.label())
                        .clicked()
                    {
                        self.save_if_dirty();
                        self.active_panel = *panel;
                    }
                }
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Vidhana v{}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                if self.dirty {
                    ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
                } else {
                    ui.label("All changes saved");
                }
            });
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.active_panel.label());
            ui.separator();

            match self.active_panel {
                Panel::Display => self.render_display(ui),
                Panel::Audio => self.render_audio(ui),
                Panel::Network => self.render_network(ui),
                Panel::Privacy => self.render_privacy(ui),
                Panel::Locale => self.render_locale(ui),
                Panel::Power => self.render_power(ui),
                Panel::Accessibility => self.render_accessibility(ui),
                Panel::History => self.render_history(ui),
                Panel::About => self.render_about(ui),
            }
        });

        // Auto-save at end of frame if dirty
        self.save_if_dirty();
    }
}

impl VidhanaApp {
    fn render_display(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut brightness = guard.display.brightness as f32;
        ui.label("Brightness");
        if ui
            .add(egui::Slider::new(&mut brightness, 0.0..=100.0).suffix("%"))
            .changed()
        {
            let old = guard.display.brightness.to_string();
            guard.display.brightness = brightness as u8;
            self.dirty = true;
            drop(guard);
            self.record_change(
                "display",
                "brightness",
                &old,
                &(brightness as u8).to_string(),
            );
            return;
        }

        ui.add_space(8.0);
        let mut theme_idx = match guard.display.theme {
            Theme::Light => 0,
            Theme::Dark => 1,
            Theme::System => 2,
        };
        let old_theme_idx = theme_idx;
        ui.label("Theme");
        egui::ComboBox::from_id_salt("theme")
            .selected_text(match theme_idx {
                0 => "Light",
                1 => "Dark",
                _ => "System",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut theme_idx, 0, "Light");
                ui.selectable_value(&mut theme_idx, 1, "Dark");
                ui.selectable_value(&mut theme_idx, 2, "System");
            });
        if theme_idx != old_theme_idx {
            let old = guard.display.theme.to_string();
            guard.display.theme = match theme_idx {
                0 => Theme::Light,
                1 => Theme::Dark,
                _ => Theme::System,
            };
            let new = guard.display.theme.to_string();
            self.dirty = true;
            drop(guard);
            self.record_change("display", "theme", &old, &new);
            return;
        }

        ui.add_space(8.0);
        let old_hc = guard.display.high_contrast;
        ui.checkbox(&mut guard.display.high_contrast, "High contrast");
        if guard.display.high_contrast != old_hc {
            self.dirty = true;
        }

        let old_nl = guard.display.night_light;
        ui.checkbox(&mut guard.display.night_light, "Night light");
        if guard.display.night_light != old_nl {
            self.dirty = true;
        }

        let mut scale = guard.display.scaling_factor as f32;
        ui.add_space(8.0);
        ui.label("Display scaling");
        if ui
            .add(egui::Slider::new(&mut scale, 0.5..=3.0).step_by(0.25))
            .changed()
        {
            guard.display.scaling_factor = scale as f64;
            self.dirty = true;
        }
    }

    fn render_audio(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut volume = guard.audio.master_volume as f32;
        ui.label("Master Volume");
        if ui
            .add(egui::Slider::new(&mut volume, 0.0..=100.0).suffix("%"))
            .changed()
        {
            guard.audio.master_volume = volume as u8;
            self.dirty = true;
        }

        let old_muted = guard.audio.muted;
        ui.checkbox(&mut guard.audio.muted, "Muted");
        if guard.audio.muted != old_muted {
            self.dirty = true;
        }

        ui.add_space(8.0);
        ui.label("Output device");
        if ui
            .text_edit_singleline(&mut guard.audio.output_device)
            .changed()
        {
            self.dirty = true;
        }

        ui.add_space(8.0);
        let mut input_vol = guard.audio.input_volume as f32;
        ui.label("Input Volume");
        if ui
            .add(egui::Slider::new(&mut input_vol, 0.0..=100.0).suffix("%"))
            .changed()
        {
            guard.audio.input_volume = input_vol as u8;
            self.dirty = true;
        }
    }

    fn render_network(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        macro_rules! toggle {
            ($field:expr, $label:expr) => {{
                let old = $field;
                ui.checkbox(&mut $field, $label);
                if $field != old {
                    self.dirty = true;
                }
            }};
        }
        toggle!(guard.network.wifi_enabled, "WiFi");
        toggle!(guard.network.bluetooth_enabled, "Bluetooth");
        toggle!(guard.network.firewall_enabled, "Firewall");
        toggle!(guard.network.vpn_enabled, "VPN");

        ui.add_space(8.0);
        ui.label("Hostname");
        if ui
            .text_edit_singleline(&mut guard.network.hostname)
            .changed()
        {
            self.dirty = true;
        }

        ui.add_space(8.0);
        ui.label("DNS Servers");
        for dns in &guard.network.dns_servers {
            ui.label(format!("  {dns}"));
        }
    }

    fn render_privacy(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let old_sl = guard.privacy.screen_lock_enabled;
        ui.checkbox(&mut guard.privacy.screen_lock_enabled, "Screen lock");
        if guard.privacy.screen_lock_enabled != old_sl {
            self.dirty = true;
        }

        let mut timeout = guard.privacy.screen_lock_timeout_secs as f32;
        ui.label("Lock timeout");
        if ui
            .add(egui::Slider::new(&mut timeout, 30.0..=3600.0).suffix("s"))
            .changed()
        {
            guard.privacy.screen_lock_timeout_secs = timeout as u32;
            self.dirty = true;
        }

        ui.add_space(8.0);
        macro_rules! toggle {
            ($field:expr, $label:expr) => {{
                let old = $field;
                ui.checkbox(&mut $field, $label);
                if $field != old {
                    self.dirty = true;
                }
            }};
        }
        toggle!(guard.privacy.location_enabled, "Location services");
        toggle!(guard.privacy.telemetry_enabled, "Telemetry");
        toggle!(guard.privacy.camera_enabled, "Camera access");
        toggle!(guard.privacy.microphone_enabled, "Microphone access");
        toggle!(
            guard.privacy.agent_approval_required,
            "Require approval for agent actions"
        );
    }

    fn render_locale(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        ui.label("Language");
        if ui
            .text_edit_singleline(&mut guard.locale.language)
            .changed()
        {
            self.dirty = true;
        }

        ui.add_space(8.0);
        ui.label("Region");
        if ui.text_edit_singleline(&mut guard.locale.region).changed() {
            self.dirty = true;
        }

        ui.add_space(8.0);
        ui.label("Timezone");
        if ui
            .text_edit_singleline(&mut guard.locale.timezone)
            .changed()
        {
            self.dirty = true;
        }

        ui.add_space(8.0);
        let old_24h = guard.locale.use_24h_clock;
        ui.checkbox(&mut guard.locale.use_24h_clock, "Use 24-hour clock");
        if guard.locale.use_24h_clock != old_24h {
            self.dirty = true;
        }

        ui.add_space(8.0);
        ui.label("Keyboard layout");
        if ui
            .text_edit_singleline(&mut guard.locale.keyboard_layout)
            .changed()
        {
            self.dirty = true;
        }
    }

    fn render_power(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut profile_idx = match guard.power.power_profile {
            PowerProfile::Performance => 0,
            PowerProfile::Balanced => 1,
            PowerProfile::PowerSaver => 2,
        };
        let old_idx = profile_idx;
        ui.label("Power profile");
        egui::ComboBox::from_id_salt("power_profile")
            .selected_text(match profile_idx {
                0 => "Performance",
                1 => "Balanced",
                _ => "Power Saver",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut profile_idx, 0, "Performance");
                ui.selectable_value(&mut profile_idx, 1, "Balanced");
                ui.selectable_value(&mut profile_idx, 2, "Power Saver");
            });
        if profile_idx != old_idx {
            guard.power.power_profile = match profile_idx {
                0 => PowerProfile::Performance,
                2 => PowerProfile::PowerSaver,
                _ => PowerProfile::Balanced,
            };
            self.dirty = true;
        }

        ui.add_space(8.0);
        let old_lid = guard.power.suspend_on_lid_close;
        ui.checkbox(
            &mut guard.power.suspend_on_lid_close,
            "Suspend on lid close",
        );
        if guard.power.suspend_on_lid_close != old_lid {
            self.dirty = true;
        }

        let mut suspend_min = guard.power.suspend_timeout_minutes as f32;
        ui.label("Suspend after");
        if ui
            .add(egui::Slider::new(&mut suspend_min, 5.0..=120.0).suffix(" min"))
            .changed()
        {
            guard.power.suspend_timeout_minutes = suspend_min as u32;
            self.dirty = true;
        }

        let mut display_min = guard.power.display_off_timeout_minutes as f32;
        ui.label("Display off after");
        if ui
            .add(egui::Slider::new(&mut display_min, 1.0..=60.0).suffix(" min"))
            .changed()
        {
            guard.power.display_off_timeout_minutes = display_min as u32;
            self.dirty = true;
        }
    }

    fn render_accessibility(&mut self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        macro_rules! toggle {
            ($field:expr, $label:expr) => {{
                let old = $field;
                ui.checkbox(&mut $field, $label);
                if $field != old {
                    self.dirty = true;
                }
            }};
        }
        toggle!(guard.accessibility.large_text, "Large text");
        toggle!(guard.accessibility.reduce_motion, "Reduce motion");
        toggle!(guard.accessibility.screen_reader, "Screen reader");
        toggle!(guard.accessibility.sticky_keys, "Sticky keys");

        ui.add_space(8.0);
        let mut cursor_idx = match guard.accessibility.cursor_size {
            CursorSize::Small => 0,
            CursorSize::Default => 1,
            CursorSize::Large => 2,
            CursorSize::ExtraLarge => 3,
        };
        let old_cursor = cursor_idx;
        ui.label("Cursor size");
        egui::ComboBox::from_id_salt("cursor_size")
            .selected_text(match cursor_idx {
                0 => "Small",
                1 => "Default",
                2 => "Large",
                _ => "Extra Large",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut cursor_idx, 0, "Small");
                ui.selectable_value(&mut cursor_idx, 1, "Default");
                ui.selectable_value(&mut cursor_idx, 2, "Large");
                ui.selectable_value(&mut cursor_idx, 3, "Extra Large");
            });
        if cursor_idx != old_cursor {
            guard.accessibility.cursor_size = match cursor_idx {
                0 => CursorSize::Small,
                2 => CursorSize::Large,
                3 => CursorSize::ExtraLarge,
                _ => CursorSize::Default,
            };
            self.dirty = true;
        }
    }

    fn render_history(&self, ui: &mut egui::Ui) {
        match self.store.recent_changes(50) {
            Ok(changes) if changes.is_empty() => {
                ui.label("No changes recorded yet.");
            }
            Ok(changes) => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("history_grid")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("Time");
                            ui.strong("Category");
                            ui.strong("Source");
                            ui.strong("Key");
                            ui.end_row();

                            for change in &changes {
                                ui.label(change.timestamp.format("%Y-%m-%d %H:%M:%S").to_string());
                                ui.label(&change.category);
                                ui.label(format!("{:?}", change.source));
                                ui.label(&change.key);
                                ui.end_row();
                            }
                        });
                });
            }
            Err(e) => {
                ui.colored_label(egui::Color32::RED, format!("Error loading history: {e}"));
            }
        }
    }

    fn render_about(&self, ui: &mut egui::Ui) {
        ui.label(format!("Vidhana v{}", env!("CARGO_PKG_VERSION")));
        ui.label("AGNOS System Settings");
        ui.add_space(8.0);
        ui.label("Sanskrit: \u{0935}\u{093F}\u{0927}\u{093E}\u{0928} (regulation, constitution, arrangement)");
        ui.add_space(16.0);
        ui.label(
            "Categories: Display, Audio, Network, Privacy, Language & Region, Power, Accessibility",
        );
        ui.add_space(8.0);
        ui.label("License: GPL-3.0");
    }
}

/// Launch the Vidhana GUI application
pub fn run_app(state: SharedState, store: Arc<SettingsStore>) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vidhana \u{2014} System Settings")
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Vidhana",
        options,
        Box::new(move |_cc| Ok(Box::new(VidhanaApp::new(state, store)))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_store() -> Arc<SettingsStore> {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-ui-test-{}-{}", std::process::id(), id));
        Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap())
    }

    #[test]
    fn test_panel_labels() {
        assert_eq!(Panel::Display.label(), "Display");
        assert_eq!(Panel::Audio.label(), "Audio");
        assert_eq!(Panel::Network.label(), "Network");
        assert_eq!(Panel::Privacy.label(), "Privacy");
        assert_eq!(Panel::Locale.label(), "Language & Region");
        assert_eq!(Panel::Power.label(), "Power");
        assert_eq!(Panel::Accessibility.label(), "Accessibility");
        assert_eq!(Panel::History.label(), "History");
        assert_eq!(Panel::About.label(), "About");
    }

    #[test]
    fn test_panel_all() {
        assert_eq!(Panel::all().len(), 9);
    }

    #[test]
    fn test_app_creation() {
        let state = new_shared_state(VidhanaConfig::default());
        let store = test_store();
        let app = VidhanaApp::new(state, store);
        assert_eq!(app.active_panel, Panel::Display);
        assert!(!app.dirty);
    }

    #[test]
    fn test_save_if_dirty() {
        let state = new_shared_state(VidhanaConfig::default());
        let store = test_store();
        let mut app = VidhanaApp::new(state.clone(), store.clone());

        // Modify and mark dirty
        state.write().unwrap().display.brightness = 42;
        app.dirty = true;
        app.save_if_dirty();
        assert!(!app.dirty);

        // Verify persisted
        let loaded = store.load_state().unwrap().unwrap();
        assert_eq!(loaded.display.brightness, 42);
    }
}
