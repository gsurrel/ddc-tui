use crate::ddc::{DdcController, MonitorInfo};
use anyhow::Result;
use ddc_hi::FeatureCode;

// VCP Feature Codes (MCCS Standard)
pub const BRIGHTNESS: FeatureCode = 0x10;
pub const CONTRAST: FeatureCode = 0x12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedElement {
    MonitorList,
    Brightness,
    Contrast,
}

pub struct App {
    pub running: bool,
    pub ddc: DdcController,
    pub monitors: Vec<MonitorInfo>,
    pub selected_monitor_idx: usize,
    pub focused_element: FocusedElement,
    pub current_brightness: u16,
    pub max_brightness: u16,
    pub current_contrast: u16,
    pub max_contrast: u16,
    pub status_message: String,
    pub is_error: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let ddc = DdcController::new()?;
        let monitors = ddc.get_monitors()?;
        let mut app = Self {
            running: true,
            monitors,
            ddc,
            selected_monitor_idx: 0,
            focused_element: FocusedElement::MonitorList,
            current_brightness: 0,
            max_brightness: 100,
            current_contrast: 0,
            max_contrast: 100,
            status_message: "Ready. Press 'r' to refresh, 'q' to quit.".to_string(),
            is_error: false,
        };

        if app.monitors.is_empty() {
            app.set_error("No DDC/CI compatible monitors found.");
        } else {
            app.refresh_values();
        }

        Ok(app)
    }

    pub fn set_error(&mut self, msg: &str) {
        self.status_message = format!("ERROR: {}", msg);
        self.is_error = true;
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.is_error = false;
    }

    pub fn refresh_values(&mut self) {
        if self.monitors.is_empty() {
            return;
        }
        let id = self.monitors[self.selected_monitor_idx].id.clone();

        match self.ddc.read_feature(&id, BRIGHTNESS) {
            Ok((val, max)) => {
                self.current_brightness = val;
                self.max_brightness = max;
            }
            Err(e) => self.set_error(&format!("Brightness read failed: {}", e)),
        }

        match self.ddc.read_feature(&id, CONTRAST) {
            Ok((val, max)) => {
                self.current_contrast = val;
                self.max_contrast = max;
            }
            Err(e) => self.set_error(&format!("Contrast read failed: {}", e)),
        }

        if !self.is_error {
            self.set_status("Values refreshed.");
        }
    }

    pub fn adjust_brightness(&mut self, delta: i16) {
        let new_val =
            (self.current_brightness as i16 + delta).clamp(0, self.max_brightness as i16) as u16;
        if new_val != self.current_brightness {
            let id = self.monitors[self.selected_monitor_idx].id.clone();
            if let Err(e) = self.ddc.write_feature(&id, BRIGHTNESS, new_val) {
                self.set_error(&format!("Write failed: {}", e));
                return;
            }
            self.current_brightness = new_val;
            self.set_status(&format!("Brightness set to {}", new_val));
        }
    }

    pub fn adjust_contrast(&mut self, delta: i16) {
        let new_val =
            (self.current_contrast as i16 + delta).clamp(0, self.max_contrast as i16) as u16;
        if new_val != self.current_contrast {
            let id = self.monitors[self.selected_monitor_idx].id.clone();
            if let Err(e) = self.ddc.write_feature(&id, CONTRAST, new_val) {
                self.set_error(&format!("Write failed: {}", e));
                return;
            }
            self.current_contrast = new_val;
            self.set_status(&format!("Contrast set to {}", new_val));
        }
    }
}
