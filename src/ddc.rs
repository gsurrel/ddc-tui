use anyhow::{Context, Result};
use ddc_hi::{Ddc, Display, FeatureCode};

pub struct DdcController;

pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub id: String, // Used to identify the monitor across re-enumerations
}

impl DdcController {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn get_monitors(&self) -> Result<Vec<MonitorInfo>> {
        let displays = Display::enumerate();
        Ok(displays
            .into_iter()
            .enumerate()
            .map(|(i, d)| {
                // Build a better fallback name if model_name is missing
                let name = d.info.model_name.clone().unwrap_or_else(|| {
                    let mut fallback = String::from("Unknown Monitor");

                    // Try to append the Manufacturer ID (e.g., "DEL" for Dell, "SAM" for Samsung)
                    if let Some(mfr) = &d.info.manufacturer_id {
                        fallback.push_str(&format!(" ({})", mfr));
                    }

                    // If we still have no useful info, use the internal device ID
                    // (which is usually an IOKit path or I2C bus address)
                    if fallback == "Unknown Monitor" {
                        fallback = format!("Monitor [{}]", d.info.id);
                    }

                    fallback
                });

                MonitorInfo {
                    index: i,
                    name,
                    id: d.info.id.clone(),
                }
            })
            .collect())
    }

    pub fn read_feature(&self, monitor_id: &str, code: FeatureCode) -> Result<(u16, u16)> {
        // Re-enumerate to get a fresh handle
        let displays = Display::enumerate();
        let mut display = displays
            .into_iter()
            .find(|d| d.info.id == monitor_id)
            .context("Monitor not found")?;

        let value = display
            .handle
            .get_vcp_feature(code)
            .with_context(|| format!("Failed to read VCP feature {}", code))?;

        // VcpValue stores max and current as high/low bytes
        let max = ((value.mh as u16) << 8) | value.ml as u16;
        let cur = ((value.sh as u16) << 8) | value.sl as u16;
        Ok((cur, max))
    }

    pub fn write_feature(&self, monitor_id: &str, code: FeatureCode, value: u16) -> Result<()> {
        let displays = Display::enumerate();
        let mut display = displays
            .into_iter()
            .find(|d| d.info.id == monitor_id)
            .context("Monitor not found")?;

        display
            .handle
            .set_vcp_feature(code, value)
            .with_context(|| format!("Failed to write VCP feature {}", code))?;
        Ok(())
    }
}
