//! Vidhana UI — egui-based system settings application
//!
//! Tabbed interface for managing all AGNOS system settings.
//! Every settings change flows through SettingsService which handles
//! validation, OS backend application, persistence, and auditing.

use std::sync::Arc;
use vidhana_backend::SettingsService;
use vidhana_core::*;
use vidhana_settings::ChangeSource;

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
    service: Arc<SettingsService>,
    active_panel: Panel,
}

impl VidhanaApp {
    pub fn new(state: SharedState, service: Arc<SettingsService>) -> Self {
        Self {
            state,
            service,
            active_panel: Panel::Display,
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
                        self.active_panel = *panel;
                    }
                }
            });

        // Bottom status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Vidhana v{}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.label("All changes saved automatically");
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
    }
}

/// Commit changes through the service if the value changed.
macro_rules! commit_if_changed {
    ($self:expr, $snapshot:expr, $current:expr, $method:ident) => {
        if $current != $snapshot {
            if let Err(e) = $self.service.$method($current, ChangeSource::Gui) {
                tracing::error!(concat!("Failed to update ", stringify!($method), ": {}"), e);
            }
        }
    };
}

impl VidhanaApp {
    fn render_display(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").display.clone();
        let mut display = snapshot.clone();

        let mut brightness = display.brightness as f32;
        ui.label("Brightness");
        if ui
            .add(egui::Slider::new(&mut brightness, 0.0..=100.0).suffix("%"))
            .changed()
        {
            display.brightness = brightness as u8;
        }

        ui.add_space(8.0);
        let mut theme_idx = match display.theme {
            Theme::Light => 0,
            Theme::Dark => 1,
            Theme::System => 2,
        };
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
        display.theme = match theme_idx {
            0 => Theme::Light,
            1 => Theme::Dark,
            _ => Theme::System,
        };

        ui.add_space(8.0);
        ui.checkbox(&mut display.high_contrast, "High contrast");
        ui.checkbox(&mut display.night_light, "Night light");

        let mut scale = display.scaling_factor as f32;
        ui.add_space(8.0);
        ui.label("Display scaling");
        if ui
            .add(egui::Slider::new(&mut scale, 0.5..=3.0).step_by(0.25))
            .changed()
        {
            display.scaling_factor = scale as f64;
        }

        commit_if_changed!(self, snapshot, display, update_display);
    }

    fn render_audio(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").audio.clone();
        let mut audio = snapshot.clone();

        let mut volume = audio.master_volume as f32;
        ui.label("Master Volume");
        if ui
            .add(egui::Slider::new(&mut volume, 0.0..=100.0).suffix("%"))
            .changed()
        {
            audio.master_volume = volume as u8;
        }

        ui.checkbox(&mut audio.muted, "Muted");

        ui.add_space(8.0);
        ui.label("Output device");
        ui.text_edit_singleline(&mut audio.output_device);

        ui.add_space(8.0);
        let mut input_vol = audio.input_volume as f32;
        ui.label("Input Volume");
        if ui
            .add(egui::Slider::new(&mut input_vol, 0.0..=100.0).suffix("%"))
            .changed()
        {
            audio.input_volume = input_vol as u8;
        }

        commit_if_changed!(self, snapshot, audio, update_audio);
    }

    fn render_network(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").network.clone();
        let mut network = snapshot.clone();

        ui.checkbox(&mut network.wifi_enabled, "WiFi");
        ui.checkbox(&mut network.bluetooth_enabled, "Bluetooth");
        ui.checkbox(&mut network.firewall_enabled, "Firewall");
        ui.checkbox(&mut network.vpn_enabled, "VPN");

        ui.add_space(8.0);
        ui.label("Hostname");
        ui.text_edit_singleline(&mut network.hostname);

        ui.add_space(8.0);
        ui.label("DNS Servers");
        for dns in &network.dns_servers {
            ui.label(format!("  {dns}"));
        }

        commit_if_changed!(self, snapshot, network, update_network);
    }

    fn render_privacy(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").privacy.clone();
        let mut privacy = snapshot.clone();

        ui.checkbox(&mut privacy.screen_lock_enabled, "Screen lock");

        let mut timeout = privacy.screen_lock_timeout_secs as f32;
        ui.label("Lock timeout");
        if ui
            .add(egui::Slider::new(&mut timeout, 30.0..=3600.0).suffix("s"))
            .changed()
        {
            privacy.screen_lock_timeout_secs = timeout as u32;
        }

        ui.add_space(8.0);
        ui.checkbox(&mut privacy.location_enabled, "Location services");
        ui.checkbox(&mut privacy.telemetry_enabled, "Telemetry");
        ui.checkbox(&mut privacy.camera_enabled, "Camera access");
        ui.checkbox(&mut privacy.microphone_enabled, "Microphone access");
        ui.checkbox(
            &mut privacy.agent_approval_required,
            "Require approval for agent actions",
        );

        commit_if_changed!(self, snapshot, privacy, update_privacy);
    }

    fn render_locale(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").locale.clone();
        let mut locale = snapshot.clone();

        ui.label("Language");
        ui.text_edit_singleline(&mut locale.language);

        ui.add_space(8.0);
        ui.label("Region");
        ui.text_edit_singleline(&mut locale.region);

        ui.add_space(8.0);
        ui.label("Timezone");
        ui.text_edit_singleline(&mut locale.timezone);

        ui.add_space(8.0);
        ui.checkbox(&mut locale.use_24h_clock, "Use 24-hour clock");

        ui.add_space(8.0);
        ui.label("Keyboard layout");
        ui.text_edit_singleline(&mut locale.keyboard_layout);

        commit_if_changed!(self, snapshot, locale, update_locale);
    }

    fn render_power(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.state.read().expect("lock poisoned").power.clone();
        let mut power = snapshot.clone();

        let mut profile_idx = match power.power_profile {
            PowerProfile::Performance => 0,
            PowerProfile::Balanced => 1,
            PowerProfile::PowerSaver => 2,
        };
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
        power.power_profile = match profile_idx {
            0 => PowerProfile::Performance,
            2 => PowerProfile::PowerSaver,
            _ => PowerProfile::Balanced,
        };

        ui.add_space(8.0);
        ui.checkbox(&mut power.suspend_on_lid_close, "Suspend on lid close");

        let mut suspend_min = power.suspend_timeout_minutes as f32;
        ui.label("Suspend after");
        if ui
            .add(egui::Slider::new(&mut suspend_min, 5.0..=120.0).suffix(" min"))
            .changed()
        {
            power.suspend_timeout_minutes = suspend_min as u32;
        }

        let mut display_min = power.display_off_timeout_minutes as f32;
        ui.label("Display off after");
        if ui
            .add(egui::Slider::new(&mut display_min, 1.0..=60.0).suffix(" min"))
            .changed()
        {
            power.display_off_timeout_minutes = display_min as u32;
        }

        commit_if_changed!(self, snapshot, power, update_power);
    }

    fn render_accessibility(&mut self, ui: &mut egui::Ui) {
        let snapshot = self
            .state
            .read()
            .expect("lock poisoned")
            .accessibility
            .clone();
        let mut a11y = snapshot.clone();

        ui.checkbox(&mut a11y.large_text, "Large text");
        ui.checkbox(&mut a11y.reduce_motion, "Reduce motion");
        ui.checkbox(&mut a11y.screen_reader, "Screen reader");
        ui.checkbox(&mut a11y.sticky_keys, "Sticky keys");

        ui.add_space(8.0);
        let mut cursor_idx = match a11y.cursor_size {
            CursorSize::Small => 0,
            CursorSize::Default => 1,
            CursorSize::Large => 2,
            CursorSize::ExtraLarge => 3,
        };
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
        a11y.cursor_size = match cursor_idx {
            0 => CursorSize::Small,
            2 => CursorSize::Large,
            3 => CursorSize::ExtraLarge,
            _ => CursorSize::Default,
        };

        commit_if_changed!(self, snapshot, a11y, update_accessibility);
    }

    fn render_history(&self, ui: &mut egui::Ui) {
        match self.service.store.recent_changes(50) {
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
        ui.label(
            "Sanskrit: \u{0935}\u{093F}\u{0927}\u{093E}\u{0928} (regulation, constitution, arrangement)",
        );
        ui.add_space(16.0);
        ui.label(
            "Categories: Display, Audio, Network, Privacy, Language & Region, Power, Accessibility",
        );
        ui.add_space(8.0);
        ui.label("License: GPL-3.0");
    }
}

/// Launch the Vidhana GUI application
pub fn run_app(state: SharedState, service: Arc<SettingsService>) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vidhana \u{2014} System Settings")
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Vidhana",
        options,
        Box::new(move |_cc| Ok(Box::new(VidhanaApp::new(state, service)))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use vidhana_backend::NoopBackend;
    use vidhana_settings::SettingsStore;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_service() -> (SharedState, Arc<SettingsService>) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("vidhana-ui-test-{}-{}", std::process::id(), id));
        let store = Arc::new(SettingsStore::new(dir.to_str().unwrap()).unwrap());
        let state = new_shared_state(VidhanaConfig::default());
        let service = Arc::new(SettingsService::new(
            state.clone(),
            store,
            Arc::new(NoopBackend),
        ));
        (state, service)
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
        let (state, service) = test_service();
        let app = VidhanaApp::new(state, service);
        assert_eq!(app.active_panel, Panel::Display);
    }
}
