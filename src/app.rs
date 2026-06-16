use crate::ddc::DdcController;
use anyhow::Result;
use ddc_hi::FeatureCode;

#[derive(Clone)]
pub struct VcpFeatureInfo {
    pub code: FeatureCode,
    pub name: String,
    pub current: u16,
    pub max: u16,
}

#[derive(Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub id: String,
    pub features: Vec<VcpFeatureInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusArea {
    MonitorList,
    VcpFeatures,
}

pub struct App {
    pub running: bool,
    pub ddc: DdcController,
    pub monitors: Vec<MonitorInfo>,
    pub selected_monitor_idx: usize,
    pub focus_area: FocusArea,
    pub selected_vcp_idx: usize,
    pub status_message: String,
    pub is_error: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let ddc = DdcController::new()?;
        let discovered = ddc.get_monitors()?;

        let monitors = discovered
            .into_iter()
            .map(|d| MonitorInfo {
                name: d.name,
                id: d.id,
                features: Vec::new(),
            })
            .collect();

        let mut app = Self {
            running: true,
            monitors,
            ddc,
            selected_monitor_idx: 0,
            focus_area: FocusArea::MonitorList,
            selected_vcp_idx: 0,
            status_message: "Ready. Press 'Tab' to switch focus, 'q' to quit.".to_string(),
            is_error: false,
        };

        if app.monitors.is_empty() {
            app.set_error("No DDC/CI compatible monitors found.");
        } else {
            app.refresh_features();
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

    pub fn refresh_features(&mut self) {
        if self.monitors.is_empty() {
            return;
        }

        // Snapshot index and id before calling into self.ddc
        let monitor_idx = self.selected_monitor_idx;
        let id = match self.monitors.get(monitor_idx) {
            Some(m) => m.id.clone(),
            None => return,
        };

        let probe_list: &[(FeatureCode, &str)] = &[
            (0x10, "Brightness"),
            (0x12, "Contrast"),
            (0x14, "Color Preset"),
            (0x16, "Red Gain"),
            (0x18, "Green Gain"),
            (0x1A, "Blue Gain"),
            (0x60, "Input Source"),
            (0x62, "Audio Volume"),
            (0x8D, "Audio Mute"),
            (0xCA, "OSD Language"),
            (0xCC, "OSD Status"),
        ];

        let mut features = Vec::new();

        // Only borrow self.ddc here
        for &(code, name) in probe_list {
            if let Ok((cur, max)) = self.ddc.read_feature(&id, code) {
                features.push(VcpFeatureInfo {
                    code,
                    name: name.to_string(),
                    current: cur,
                    max,
                });
            }
        }

        // Mutate monitors after DDC calls
        if let Some(mon) = self.monitors.get_mut(monitor_idx) {
            mon.features = features;
        }
        self.set_status("Features refreshed.");
    }

    pub fn adjust_selected_feature(&mut self, delta: i16) {
        if self.monitors.is_empty() {
            return;
        }

        let monitor_idx = self.selected_monitor_idx;
        let vcp_idx = self.selected_vcp_idx;

        // Snapshot id and feature data (no long-lived borrow)
        let id = match self.monitors.get(monitor_idx) {
            Some(m) => m.id.clone(),
            None => return,
        };

        let (current, max, code, name) = {
            let feature_ref = match self
                .monitors
                .get(monitor_idx)
                .and_then(|m| m.features.get(vcp_idx))
            {
                Some(f) => f,
                None => return,
            };
            (
                feature_ref.current,
                feature_ref.max,
                feature_ref.code,
                feature_ref.name.clone(),
            )
        };

        if max == 0 {
            return;
        }

        let current_i32 = i32::from(current);
        let max_i32 = i32::from(max);
        let new_val = (current_i32 + i32::from(delta)).clamp(0, max_i32);

        let final_val = match u16::try_from(new_val) {
            Ok(v) => v,
            Err(_) => {
                self.set_error("Value out of bounds");
                return;
            }
        };

        if final_val == current {
            return;
        }

        // Only borrow self.ddc here
        if self.ddc.write_feature(&id, code, final_val).is_ok() {
            // Re-borrow monitors mutably to update cached value
            if let Some(feature_mut) = self
                .monitors
                .get_mut(monitor_idx)
                .and_then(|m| m.features.get_mut(vcp_idx))
            {
                feature_mut.current = final_val;
            }
            self.set_status(&format!("Set {} to {}", name, final_val));
        } else {
            self.set_error("Write failed");
        }
    }
}
