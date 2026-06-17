use crate::db::{BASE_CONTROLS, MONITOR_PROFILES};
use crate::ddc::DdcController;
use anyhow::Result;
use ddc_hi::FeatureCode;

#[derive(Clone)]
pub struct VcpFeatureInfo {
    pub code: FeatureCode,
    pub name: String,
    pub current: u16,
    pub max: u16,
    pub is_discrete: bool,
    pub options: Vec<String>,
    pub option_values: Vec<u16>,
}

#[derive(Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub id: String,
    pub manufacturer_id: Option<String>,
    pub model_id: Option<u16>,
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
                manufacturer_id: d.manufacturer_id,
                model_id: d.model_id,
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
        let monitor_idx = self.selected_monitor_idx;
        let m = match self.monitors.get(monitor_idx) {
            Some(m) => m,
            None => return,
        };

        // Heuristic: Exact Match -> Manufacturer LCD -> VESA
        let exact_pnp = match (&m.manufacturer_id, m.model_id) {
            (Some(mfr), Some(model)) => format!("{}{:04X}", mfr, model),
            _ => String::new(),
        };
        let mfr_lcd = match &m.manufacturer_id {
            Some(mfr) => format!("{}lcd", mfr),
            None => String::new(),
        };

        let profile = MONITOR_PROFILES
            .iter()
            .find(|p| p.pnp_name == exact_pnp)
            .or_else(|| MONITOR_PROFILES.iter().find(|p| p.pnp_name == mfr_lcd))
            .or_else(|| MONITOR_PROFILES.iter().find(|p| p.pnp_name == "VESA"))
            .unwrap();

        let mut features = Vec::new();
        for base_ctrl in BASE_CONTROLS {
            let override_opt = profile
                .overrides
                .iter()
                .find(|o| o.control_id == base_ctrl.id);
            let address = override_opt
                .and_then(|o| o.address_override)
                .unwrap_or(base_ctrl.address);

            let resolved_values = if let Some(o) = override_opt {
                if !o.values.is_empty() {
                    o.values
                } else {
                    base_ctrl.default_values
                }
            } else {
                base_ctrl.default_values
            };

            if let Ok((cur, max)) = self.ddc.read_feature(&m.id, address) {
                let mut options = Vec::new();
                let mut option_values = Vec::new();
                let is_discrete = !resolved_values.is_empty() || max <= 4;

                if !resolved_values.is_empty() {
                    for &(val, name) in resolved_values {
                        options.push(name.to_string());
                        option_values.push(val);
                    }
                } else if max <= 4 {
                    for i in 0..=max {
                        options.push(format!("{}", i));
                        option_values.push(i);
                    }
                }

                features.push(VcpFeatureInfo {
                    code: address,
                    name: base_ctrl.name.to_string(),
                    current: cur,
                    max,
                    is_discrete,
                    options,
                    option_values,
                });
            }
        }
        if let Some(mon) = self.monitors.get_mut(monitor_idx) {
            mon.features = features;
        }
        self.set_status(&format!("Loaded profile: {}", profile.display_name));
    }

    pub fn adjust_selected_feature(&mut self, delta: i16) {
        let monitor_idx = self.selected_monitor_idx;
        let vcp_idx = self.selected_vcp_idx;
        let id = match self.monitors.get(monitor_idx) {
            Some(m) => m.id.clone(),
            None => return,
        };

        let (current, max, code, name, is_discrete, option_values) = {
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
                feature_ref.is_discrete,
                feature_ref.option_values.clone(),
            )
        };

        if max == 0 {
            return;
        }

        let final_val = if is_discrete && !option_values.is_empty() {
            let current_pos = option_values
                .iter()
                .position(|&v| v == current)
                .unwrap_or(0);
            let new_pos = (current_pos as i32 + i32::from(delta))
                .clamp(0, option_values.len() as i32 - 1) as usize;
            option_values[new_pos]
        } else {
            let new_val = (i32::from(current) + i32::from(delta)).clamp(0, i32::from(max));
            match u16::try_from(new_val) {
                Ok(v) => v,
                Err(_) => {
                    self.set_error("Value out of bounds");
                    return;
                }
            }
        };

        if final_val == current {
            return;
        }

        if self.ddc.write_feature(&id, code, final_val).is_ok() {
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
