use anyhow::{Context, Result};
use ddc_hi::{Ddc, Display, FeatureCode};

pub struct DdcController;

pub struct DiscoveredMonitor {
    pub name: String,
    pub id: String,
}

impl DdcController {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn get_monitors(&self) -> Result<Vec<DiscoveredMonitor>> {
        let displays = Display::enumerate();
        Ok(displays
            .into_iter()
            .map(|d| {
                let name = d.info.model_name.clone().unwrap_or_else(|| {
                    let mut fallback = String::from("Unknown Monitor");
                    if let Some(mfr) = &d.info.manufacturer_id {
                        fallback.push_str(&format!(" ({})", mfr));
                    }
                    if fallback == "Unknown Monitor" {
                        fallback = format!("Monitor [{}]", d.info.id);
                    }
                    fallback
                });

                DiscoveredMonitor {
                    name,
                    id: d.info.id.clone(),
                }
            })
            .collect())
    }

    pub fn read_feature(&self, monitor_id: &str, code: FeatureCode) -> Result<(u16, u16)> {
        let displays = Display::enumerate();
        let mut display = displays
            .into_iter()
            .find(|d| d.info.id == monitor_id)
            .context("Monitor not found")?;

        let value = display
            .handle
            .get_vcp_feature(code)
            .with_context(|| format!("Failed to read VCP feature {}", code))?;

        // Safe bitwise combination using From instead of `as`
        let max = (u16::from(value.mh) << 8) | u16::from(value.ml);
        let cur = (u16::from(value.sh) << 8) | u16::from(value.sl);
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
