use crate::db::{BASE_CONTROLS, MONITOR_PROFILES};
use crate::ddc::DdcController;
use anyhow::Result;
use ddc_hi::FeatureCode;
use std::collections::HashSet;

#[derive(Clone)]
pub enum FeatureType {
    Continuous {
        max: u16,
    },
    Discrete {
        options: Vec<String>,
        values: Vec<u16>,
    },
    ActionGroup {
        actions: Vec<VcpFeatureInfo>,
    },
}

#[derive(Clone)]
pub struct VcpFeatureInfo {
    pub code: FeatureCode,
    pub name: String,
    pub current: u16,
    pub feature_type: FeatureType,
}

#[derive(Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub id: String,
    pub manufacturer_id: Option<String>,
    pub model_id: Option<u16>,
    pub features: Vec<VcpFeatureInfo>,
    pub profile_chain: Vec<String>,
    pub override_profile_pnp: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusArea {
    MonitorList,
    VcpFeatures,
}

#[derive(Clone, Copy, PartialEq)]
pub enum UIMode {
    Normal,
    ProfileSearch,
}

pub struct App {
    pub running: bool,
    pub ddc: DdcController,
    pub monitors: Vec<MonitorInfo>,
    pub selected_monitor_idx: usize,
    pub focus_area: FocusArea,
    pub selected_vcp_idx: usize,
    pub selected_pill_idx: usize,
    pub scroll_offset: usize,
    pub status_message: String,
    pub is_error: bool,

    pub ui_mode: UIMode,
    pub is_probing: bool,
    pub pending_probe: bool,

    pub search_query: String,
    pub search_results: Vec<usize>,
    pub search_selected_idx: usize,
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
                profile_chain: Vec::new(),
                override_profile_pnp: None,
            })
            .collect();

        let mut app = Self {
            running: true,
            monitors,
            ddc,
            selected_monitor_idx: 0,
            focus_area: FocusArea::MonitorList,
            selected_vcp_idx: 0,
            selected_pill_idx: 0,
            scroll_offset: 0,
            status_message: "Ready.".to_string(),
            is_error: false,
            ui_mode: UIMode::Normal,
            is_probing: false,
            pending_probe: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected_idx: 0,
        };

        if app.monitors.is_empty() {
            app.set_error("No DDC/CI monitors found.");
        } else {
            app.start_probe();
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

    pub fn start_probe(&mut self) {
        self.is_probing = true;
        self.pending_probe = true;
    }

    pub fn enter_search_mode(&mut self) {
        self.ui_mode = UIMode::ProfileSearch;
        self.search_query = String::new();
        self.update_search_results();
    }

    pub fn update_search_results(&mut self) {
        let query = self.search_query.to_lowercase();
        let mut scored: Vec<(f64, usize)> = MONITOR_PROFILES
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let score1 = strsim::jaro_winkler(&query, &p.display_name.to_lowercase());
                let score2 = strsim::jaro_winkler(&query, &p.pnp_name.to_lowercase());
                (score1.max(score2), i)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.search_results = scored.into_iter().map(|(_, i)| i).collect();
        self.search_selected_idx = 0;
    }

    pub fn apply_search_selection(&mut self) {
        if let Some(&profile_idx) = self.search_results.get(self.search_selected_idx) {
            let pnp = MONITOR_PROFILES[profile_idx].pnp_name.to_string();
            if let Some(m) = self.monitors.get_mut(self.selected_monitor_idx) {
                m.override_profile_pnp = Some(pnp);
            }
        }
        self.ui_mode = UIMode::Normal;
        self.start_probe();
    }

    pub fn update_pill_idx_to_current(&mut self) {
        if let Some(m) = self.monitors.get(self.selected_monitor_idx) {
            if let Some(f) = m.features.get(self.selected_vcp_idx) {
                match &f.feature_type {
                    FeatureType::Discrete { values, .. } => {
                        let idx = values.iter().position(|&v| v == f.current).unwrap_or(0);
                        self.selected_pill_idx = idx.min(values.len().saturating_sub(1));
                    }
                    FeatureType::ActionGroup { actions } => {
                        self.selected_pill_idx =
                            self.selected_pill_idx.min(actions.len().saturating_sub(1));
                    }
                    FeatureType::Continuous { .. } => {}
                }
            }
        } else {
            self.selected_pill_idx = 0;
        }
    }

    pub fn navigate_left(&mut self, shift_held: bool) {
        if let Some(m) = self.monitors.get(self.selected_monitor_idx) {
            if let Some(f) = m.features.get(self.selected_vcp_idx) {
                match &f.feature_type {
                    FeatureType::Continuous { .. } => self.adjust_gauge(-1, shift_held),
                    FeatureType::Discrete { .. } | FeatureType::ActionGroup { .. } => {
                        self.move_pill_selection(-1)
                    }
                }
            }
        }
    }

    pub fn navigate_right(&mut self, shift_held: bool) {
        if let Some(m) = self.monitors.get(self.selected_monitor_idx) {
            if let Some(f) = m.features.get(self.selected_vcp_idx) {
                match &f.feature_type {
                    FeatureType::Continuous { .. } => self.adjust_gauge(1, shift_held),
                    FeatureType::Discrete { .. } | FeatureType::ActionGroup { .. } => {
                        self.move_pill_selection(1)
                    }
                }
            }
        }
    }

    pub fn move_pill_selection(&mut self, delta: i16) {
        if let Some(m) = self.monitors.get(self.selected_monitor_idx) {
            if let Some(f) = m.features.get(self.selected_vcp_idx) {
                let max_idx = match &f.feature_type {
                    FeatureType::Discrete { values, .. } => values.len() as i16 - 1,
                    FeatureType::ActionGroup { actions } => actions.len() as i16 - 1,
                    _ => return,
                };
                if max_idx < 0 {
                    return;
                }
                let new_idx = (self.selected_pill_idx as i16 + delta).clamp(0, max_idx) as usize;
                self.selected_pill_idx = new_idx;
            }
        }
    }

    pub fn commit_pill_selection(&mut self) {
        let monitor_idx = self.selected_monitor_idx;
        let vcp_idx = self.selected_vcp_idx;
        let pill_idx = self.selected_pill_idx;

        let (id, code, name, feature_type) = {
            let m = match self.monitors.get(monitor_idx) {
                Some(m) => m,
                None => return,
            };
            let f = match m.features.get(vcp_idx) {
                Some(f) => f,
                None => return,
            };
            (m.id.clone(), f.code, f.name.clone(), f.feature_type.clone())
        };

        match feature_type {
            FeatureType::Discrete { values, .. } => {
                if let Some(&val) = values.get(pill_idx) {
                    if let Some(f) = self
                        .monitors
                        .get_mut(monitor_idx)
                        .and_then(|m| m.features.get_mut(vcp_idx))
                    {
                        f.current = val;
                    }
                    if let Ok(mut display) = self.ddc.find_display(&id) {
                        if DdcController::write_feature(&mut display, code, val).is_ok() {
                            self.set_status(&format!("Set {} to {}", name, val));
                        } else {
                            self.set_error("Write failed");
                        }
                    }
                }
            }
            FeatureType::ActionGroup { actions } => {
                if let Some(action) = actions.get(pill_idx) {
                    let action_code = action.code;
                    let action_name = action.name.clone();
                    if let Ok(mut display) = self.ddc.find_display(&id) {
                        if DdcController::write_feature(&mut display, action_code, 1).is_ok() {
                            self.set_status(&format!("Executed {}", action_name));
                        } else {
                            self.set_error("Action failed");
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn adjust_gauge(&mut self, delta: i16, shift_held: bool) {
        let monitor_idx = self.selected_monitor_idx;
        let vcp_idx = self.selected_vcp_idx;

        let (id, code, name, feature_type, current) = {
            let m = match self.monitors.get(monitor_idx) {
                Some(m) => m,
                None => return,
            };
            let f = match m.features.get(vcp_idx) {
                Some(f) => f,
                None => return,
            };
            (
                m.id.clone(),
                f.code,
                f.name.clone(),
                f.feature_type.clone(),
                f.current,
            )
        };

        if let FeatureType::Continuous { max } = feature_type {
            let step = if shift_held {
                ((max as i32) / 10).max(1)
            } else {
                1
            };
            let new_val = (current as i32 + delta as i32 * step).clamp(0, max as i32);
            let final_val = u16::try_from(new_val).unwrap_or(current);

            if final_val == current {
                return;
            }

            if let Some(f) = self
                .monitors
                .get_mut(monitor_idx)
                .and_then(|m| m.features.get_mut(vcp_idx))
            {
                f.current = final_val;
            }

            if let Ok(mut display) = self.ddc.find_display(&id) {
                if DdcController::write_feature(&mut display, code, final_val).is_ok() {
                    self.set_status(&format!("Set {} to {}", name, final_val));
                } else {
                    self.set_error("Write failed");
                }
            }
        }
    }

    pub fn execute_probe(&mut self) {
        let monitor_idx = self.selected_monitor_idx;
        if self.monitors.is_empty() || monitor_idx >= self.monitors.len() {
            self.is_probing = false;
            return;
        }

        let (id, override_pnp, mfr_id, model_id) = {
            let m = &self.monitors[monitor_idx];
            (
                m.id.clone(),
                m.override_profile_pnp.clone(),
                m.manufacturer_id.clone(),
                m.model_id,
            )
        };

        let profile_pnp = override_pnp.unwrap_or_else(|| {
            let exact = match (&mfr_id, model_id) {
                (Some(m), Some(id)) => format!("{}{:04X}", m, id),
                _ => String::new(),
            };
            let mfr_lcd = mfr_id
                .as_ref()
                .map(|m| format!("{}lcd", m))
                .unwrap_or_default();
            for cand in [exact.as_str(), mfr_lcd.as_str(), "VESA"] {
                if !cand.is_empty() && MONITOR_PROFILES.iter().any(|p| p.pnp_name == cand) {
                    return cand.to_string();
                }
            }
            "VESA".to_string()
        });

        let mut chain = vec![profile_pnp.clone()];
        let mut queue = Vec::new();
        if let Some(p) = MONITOR_PROFILES.iter().find(|p| p.pnp_name == profile_pnp) {
            queue.extend(p.includes.iter().map(|s| s.to_string()));
        }
        while let Some(inc) = queue.pop() {
            if !chain.contains(&inc) {
                chain.push(inc.clone());
                if let Some(inc_p) = MONITOR_PROFILES.iter().find(|p| p.pnp_name == inc) {
                    queue.extend(inc_p.includes.iter().map(|s| s.to_string()));
                }
            }
        }

        let mut supported_ids = HashSet::new();
        for pnp in &chain {
            if let Some(p) = MONITOR_PROFILES.iter().find(|prof| prof.pnp_name == pnp) {
                for o in p.overrides {
                    supported_ids.insert(o.control_id);
                }
            }
        }

        let mut normal_features = Vec::new();
        let mut action_features = Vec::new();

        if let Ok(mut display) = self.ddc.find_display(&id) {
            for base_ctrl in BASE_CONTROLS {
                if !supported_ids.contains(base_ctrl.id) {
                    continue;
                }
                let mut resolved_addr = base_ctrl.address;
                let mut resolved_values = base_ctrl.default_values;

                for pnp in &chain {
                    if let Some(p) = MONITOR_PROFILES.iter().find(|prof| prof.pnp_name == pnp) {
                        if let Some(o) = p.overrides.iter().find(|o| o.control_id == base_ctrl.id) {
                            if let Some(addr) = o.address_override {
                                resolved_addr = addr;
                            }
                            if !o.values.is_empty() {
                                resolved_values = o.values;
                            }
                            break;
                        }
                    }
                }

                if let Ok((cur, max)) = DdcController::read_feature(&mut display, resolved_addr) {
                    let is_action = base_ctrl.name.contains("Restore")
                        || base_ctrl.name.contains("Reset")
                        || base_ctrl.name.contains("Degauss")
                        || (max <= 1 && resolved_values.is_empty() && !base_ctrl.is_list);

                    if is_action {
                        action_features.push(VcpFeatureInfo {
                            code: resolved_addr,
                            name: base_ctrl.name.to_string(),
                            current: cur,
                            feature_type: FeatureType::ActionGroup {
                                actions: Vec::new(),
                            }, // Temp
                        });
                    } else if !resolved_values.is_empty() || max <= 4 {
                        let mut options = Vec::new();
                        let mut values = Vec::new();
                        if !resolved_values.is_empty() {
                            for &(val, name) in resolved_values {
                                options.push(name.to_string());
                                values.push(val);
                            }
                        } else {
                            for i in 0..=max {
                                options.push(format!("{}", i));
                                values.push(i);
                            }
                        }
                        normal_features.push(VcpFeatureInfo {
                            code: resolved_addr,
                            name: base_ctrl.name.to_string(),
                            current: cur,
                            feature_type: FeatureType::Discrete { options, values },
                        });
                    } else {
                        normal_features.push(VcpFeatureInfo {
                            code: resolved_addr,
                            name: base_ctrl.name.to_string(),
                            current: cur,
                            feature_type: FeatureType::Continuous { max },
                        });
                    }
                }
            }
        }

        let mut features = normal_features;

        // Group all actions into a single control at the end
        if !action_features.is_empty() {
            features.push(VcpFeatureInfo {
                code: 0,
                name: "Actions".to_string(),
                current: 0,
                feature_type: FeatureType::ActionGroup {
                    actions: action_features,
                },
            });
        }

        if let Some(mon) = self.monitors.get_mut(monitor_idx) {
            mon.features = features;
            mon.profile_chain = chain;
        }

        self.update_pill_idx_to_current();
        self.is_probing = false;
        self.set_status("Probe complete.");
    }
}
