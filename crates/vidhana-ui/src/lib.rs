//! Vidhana UI — egui-based system settings application
//!
//! Tabbed interface for managing all AGNOS system settings.

use vidhana_core::*;

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
            Self::About => "About",
        }
    }

    pub fn all() -> &'static [Panel] {
        &[
            Self::Display, Self::Audio, Self::Network, Self::Privacy,
            Self::Locale, Self::Power, Self::Accessibility, Self::About,
        ]
    }
}

/// Main Vidhana application
pub struct VidhanaApp {
    state: SharedState,
    active_panel: Panel,
}

impl VidhanaApp {
    pub fn new(state: SharedState) -> Self {
        Self {
            state,
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
                    if ui.selectable_label(self.active_panel == *panel, panel.label()).clicked() {
                        self.active_panel = *panel;
                    }
                }
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
                Panel::About => self.render_about(ui),
            }
        });
    }
}

impl VidhanaApp {
    fn render_display(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut brightness = guard.display.brightness as f32;
        ui.label("Brightness");
        if ui.add(egui::Slider::new(&mut brightness, 0.0..=100.0).suffix("%")).changed() {
            guard.display.brightness = brightness as u8;
        }

        ui.add_space(8.0);
        let mut theme_idx = match guard.display.theme {
            Theme::Light => 0,
            Theme::Dark => 1,
            Theme::System => 2,
        };
        ui.label("Theme");
        egui::ComboBox::from_id_salt("theme")
            .selected_text(match theme_idx { 0 => "Light", 1 => "Dark", _ => "System" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut theme_idx, 0, "Light");
                ui.selectable_value(&mut theme_idx, 1, "Dark");
                ui.selectable_value(&mut theme_idx, 2, "System");
            });
        guard.display.theme = match theme_idx { 0 => Theme::Light, 1 => Theme::Dark, _ => Theme::System };

        ui.add_space(8.0);
        ui.checkbox(&mut guard.display.high_contrast, "High contrast");
        ui.checkbox(&mut guard.display.night_light, "Night light");

        let mut scale = guard.display.scaling_factor as f32;
        ui.add_space(8.0);
        ui.label("Display scaling");
        if ui.add(egui::Slider::new(&mut scale, 0.5..=3.0).step_by(0.25)).changed() {
            guard.display.scaling_factor = scale as f64;
        }
    }

    fn render_audio(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut volume = guard.audio.master_volume as f32;
        ui.label("Master Volume");
        if ui.add(egui::Slider::new(&mut volume, 0.0..=100.0).suffix("%")).changed() {
            guard.audio.master_volume = volume as u8;
        }

        ui.checkbox(&mut guard.audio.muted, "Muted");

        ui.add_space(8.0);
        ui.label("Output device");
        ui.text_edit_singleline(&mut guard.audio.output_device);

        ui.add_space(8.0);
        let mut input_vol = guard.audio.input_volume as f32;
        ui.label("Input Volume");
        if ui.add(egui::Slider::new(&mut input_vol, 0.0..=100.0).suffix("%")).changed() {
            guard.audio.input_volume = input_vol as u8;
        }
    }

    fn render_network(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        ui.checkbox(&mut guard.network.wifi_enabled, "WiFi");
        ui.checkbox(&mut guard.network.bluetooth_enabled, "Bluetooth");
        ui.checkbox(&mut guard.network.firewall_enabled, "Firewall");
        ui.checkbox(&mut guard.network.vpn_enabled, "VPN");

        ui.add_space(8.0);
        ui.label("Hostname");
        ui.text_edit_singleline(&mut guard.network.hostname);

        ui.add_space(8.0);
        ui.label("DNS Servers");
        for dns in &guard.network.dns_servers {
            ui.label(format!("  {dns}"));
        }
    }

    fn render_privacy(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        ui.checkbox(&mut guard.privacy.screen_lock_enabled, "Screen lock");

        let mut timeout = guard.privacy.screen_lock_timeout_secs as f32;
        ui.label("Lock timeout");
        if ui.add(egui::Slider::new(&mut timeout, 30.0..=3600.0).suffix("s")).changed() {
            guard.privacy.screen_lock_timeout_secs = timeout as u32;
        }

        ui.add_space(8.0);
        ui.checkbox(&mut guard.privacy.location_enabled, "Location services");
        ui.checkbox(&mut guard.privacy.telemetry_enabled, "Telemetry");
        ui.checkbox(&mut guard.privacy.camera_enabled, "Camera access");
        ui.checkbox(&mut guard.privacy.microphone_enabled, "Microphone access");
        ui.checkbox(&mut guard.privacy.agent_approval_required, "Require approval for agent actions");
    }

    fn render_locale(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        ui.label("Language");
        ui.text_edit_singleline(&mut guard.locale.language);

        ui.add_space(8.0);
        ui.label("Region");
        ui.text_edit_singleline(&mut guard.locale.region);

        ui.add_space(8.0);
        ui.label("Timezone");
        ui.text_edit_singleline(&mut guard.locale.timezone);

        ui.add_space(8.0);
        ui.checkbox(&mut guard.locale.use_24h_clock, "Use 24-hour clock");

        ui.add_space(8.0);
        ui.label("Keyboard layout");
        ui.text_edit_singleline(&mut guard.locale.keyboard_layout);
    }

    fn render_power(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        let mut profile_idx = match guard.power.power_profile {
            PowerProfile::Performance => 0,
            PowerProfile::Balanced => 1,
            PowerProfile::PowerSaver => 2,
        };
        ui.label("Power profile");
        egui::ComboBox::from_id_salt("power_profile")
            .selected_text(match profile_idx { 0 => "Performance", 1 => "Balanced", _ => "Power Saver" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut profile_idx, 0, "Performance");
                ui.selectable_value(&mut profile_idx, 1, "Balanced");
                ui.selectable_value(&mut profile_idx, 2, "Power Saver");
            });
        guard.power.power_profile = match profile_idx {
            0 => PowerProfile::Performance,
            2 => PowerProfile::PowerSaver,
            _ => PowerProfile::Balanced,
        };

        ui.add_space(8.0);
        ui.checkbox(&mut guard.power.suspend_on_lid_close, "Suspend on lid close");

        let mut suspend_min = guard.power.suspend_timeout_minutes as f32;
        ui.label("Suspend after");
        if ui.add(egui::Slider::new(&mut suspend_min, 5.0..=120.0).suffix(" min")).changed() {
            guard.power.suspend_timeout_minutes = suspend_min as u32;
        }

        let mut display_min = guard.power.display_off_timeout_minutes as f32;
        ui.label("Display off after");
        if ui.add(egui::Slider::new(&mut display_min, 1.0..=60.0).suffix(" min")).changed() {
            guard.power.display_off_timeout_minutes = display_min as u32;
        }
    }

    fn render_accessibility(&self, ui: &mut egui::Ui) {
        let mut guard = self.state.write().unwrap();

        ui.checkbox(&mut guard.accessibility.large_text, "Large text");
        ui.checkbox(&mut guard.accessibility.reduce_motion, "Reduce motion");
        ui.checkbox(&mut guard.accessibility.screen_reader, "Screen reader");
        ui.checkbox(&mut guard.accessibility.sticky_keys, "Sticky keys");

        ui.add_space(8.0);
        let mut cursor_idx = match guard.accessibility.cursor_size {
            CursorSize::Small => 0,
            CursorSize::Default => 1,
            CursorSize::Large => 2,
            CursorSize::ExtraLarge => 3,
        };
        ui.label("Cursor size");
        egui::ComboBox::from_id_salt("cursor_size")
            .selected_text(match cursor_idx { 0 => "Small", 1 => "Default", 2 => "Large", _ => "Extra Large" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut cursor_idx, 0, "Small");
                ui.selectable_value(&mut cursor_idx, 1, "Default");
                ui.selectable_value(&mut cursor_idx, 2, "Large");
                ui.selectable_value(&mut cursor_idx, 3, "Extra Large");
            });
        guard.accessibility.cursor_size = match cursor_idx {
            0 => CursorSize::Small,
            2 => CursorSize::Large,
            3 => CursorSize::ExtraLarge,
            _ => CursorSize::Default,
        };
    }

    fn render_about(&self, ui: &mut egui::Ui) {
        ui.label(format!("Vidhana v{}", env!("CARGO_PKG_VERSION")));
        ui.label("AGNOS System Settings");
        ui.add_space(8.0);
        ui.label("Sanskrit: विधान (regulation, constitution, arrangement)");
        ui.add_space(16.0);
        ui.label("Categories: Display, Audio, Network, Privacy, Language & Region, Power, Accessibility");
        ui.add_space(8.0);
        ui.label("License: GPL-3.0");
    }
}

/// Launch the Vidhana GUI application
pub fn run_app(state: SharedState) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Vidhana — System Settings")
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Vidhana",
        options,
        Box::new(move |_cc| Ok(Box::new(VidhanaApp::new(state)))),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_labels() {
        assert_eq!(Panel::Display.label(), "Display");
        assert_eq!(Panel::Audio.label(), "Audio");
        assert_eq!(Panel::Network.label(), "Network");
        assert_eq!(Panel::Privacy.label(), "Privacy");
        assert_eq!(Panel::Locale.label(), "Language & Region");
        assert_eq!(Panel::Power.label(), "Power");
        assert_eq!(Panel::Accessibility.label(), "Accessibility");
        assert_eq!(Panel::About.label(), "About");
    }

    #[test]
    fn test_panel_all() {
        assert_eq!(Panel::all().len(), 8);
    }

    #[test]
    fn test_app_creation() {
        let state = new_shared_state(VidhanaConfig::default());
        let app = VidhanaApp::new(state);
        assert_eq!(app.active_panel, Panel::Display);
    }
}
