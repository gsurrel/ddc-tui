use roxmltree::Document;
use std::fs;
use std::io::Write;
use std::path::Path;

struct BaseCtrl {
    id: String,
    name: String,
    addr: u8,
    is_list: bool,
    vals: Vec<(u16, String)>,
}
struct Override {
    id: String,
    addr: Option<u8>,
    vals: Vec<(u16, String)>,
}
struct Profile {
    pnp: String,
    name: String,
    overrides: Vec<Override>,
    includes: Vec<String>,
}

fn parse_hex(s: &str) -> Result<u16, ()> {
    let s = s.trim();
    if s.is_empty() {
        return Err(());
    }
    if s.starts_with("0x") || s.starts_with("0X") {
        u16::from_str_radix(&s[2..], 16).map_err(|_| ())
    } else {
        s.parse::<u16>().map_err(|_| ())
    }
}

fn extract_overrides(doc: &Document) -> Vec<Override> {
    let mut overrides = Vec::new();
    for node in doc.descendants() {
        if node.tag_name().name() == "control" {
            let id = node.attribute("id").unwrap_or("");
            let addr = parse_hex(node.attribute("address").unwrap_or(""))
                .ok()
                .map(|a| a as u8);
            let mut vals = Vec::new();
            for v in node.children() {
                if v.tag_name().name() == "value" {
                    let v_name = v
                        .attribute("name")
                        .unwrap_or(v.attribute("id").unwrap_or(""));
                    if let Ok(v_hex) = parse_hex(v.attribute("value").unwrap_or("")) {
                        vals.push((v_hex, v_name.to_string()));
                    }
                }
            }
            if addr.is_some() || !vals.is_empty() {
                overrides.push(Override {
                    id: id.to_string(),
                    addr,
                    vals,
                });
            }
        }
    }
    overrides
}

fn main() {
    println!("cargo:rerun-if-changed=ddccontrol-db");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("db_generated.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    let db_dir = Path::new("ddccontrol-db/db");
    if !db_dir.exists() {
        println!(
            "cargo:warning=ddccontrol-db submodule not found. Run `git submodule update --init`"
        );
        writeln!(
            f,
            "pub const BASE_CONTROLS: &[crate::db::BaseControl] = &[];"
        )
        .unwrap();
        writeln!(
            f,
            "pub const MONITOR_PROFILES: &[crate::db::MonitorProfile] = &[];"
        )
        .unwrap();
        return;
    }

    let mut base_controls = Vec::new();
    if let Ok(xml) = fs::read_to_string(db_dir.join("options.xml.in")) {
        if let Ok(doc) = Document::parse(&xml) {
            for node in doc.descendants() {
                if node.tag_name().name() == "control" {
                    let id = node.attribute("id").unwrap_or("");
                    let name = node.attribute("name").unwrap_or(id);
                    let typ = node.attribute("type").unwrap_or("value");
                    if let Ok(addr) = parse_hex(node.attribute("address").unwrap_or("")) {
                        let mut vals = Vec::new();
                        for v in node.children() {
                            if v.tag_name().name() == "value" {
                                let v_name = v
                                    .attribute("name")
                                    .unwrap_or(v.attribute("id").unwrap_or(""));
                                if let Ok(v_hex) = parse_hex(v.attribute("value").unwrap_or("")) {
                                    vals.push((v_hex, v_name.to_string()));
                                }
                            }
                        }
                        base_controls.push(BaseCtrl {
                            id: id.to_string(),
                            name: name.to_string(),
                            addr: addr as u8,
                            is_list: typ == "list",
                            vals,
                        });
                    }
                }
            }
        }
    }

    let mut profiles = Vec::new();
    let monitor_dir = db_dir.join("monitor");
    if monitor_dir.exists() {
        if let Ok(entries) = fs::read_dir(&monitor_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                    let pnp = path.file_stem().unwrap().to_str().unwrap().to_string();
                    if let Ok(xml) = fs::read_to_string(&path) {
                        if let Ok(doc) = Document::parse(&xml) {
                            let mon_name = doc
                                .root_element()
                                .attribute("name")
                                .unwrap_or(&pnp)
                                .to_string();
                            let overrides = extract_overrides(&doc);

                            // Extract include chain
                            let mut includes = Vec::new();
                            for node in doc.descendants() {
                                if node.tag_name().name() == "include" {
                                    if let Some(file) = node.attribute("file") {
                                        includes.push(file.to_string());
                                    }
                                }
                            }
                            profiles.push(Profile {
                                pnp,
                                name: mon_name,
                                overrides,
                                includes,
                            });
                        }
                    }
                }
            }
        }
    }

    // Generate Rust Code
    writeln!(f, "#[allow(dead_code)] pub struct BaseControl {{ pub id: &'static str, pub name: &'static str, pub address: u8, pub is_list: bool, pub default_values: &'static [(u16, &'static str)] }}").unwrap();
    writeln!(f, "pub const BASE_CONTROLS: &[BaseControl] = &[").unwrap();
    for ctrl in &base_controls {
        let vals_str: Vec<String> = ctrl
            .vals
            .iter()
            .map(|(v, n)| format!("(0x{:04X}, \"{}\")", v, n.replace("\"", "\\\"")))
            .collect();
        writeln!(f, "    BaseControl {{ id: \"{}\", name: \"{}\", address: 0x{:02X}, is_list: {}, default_values: &[{}] }},", ctrl.id.replace("\"", "\\\""), ctrl.name.replace("\"", "\\\""), ctrl.addr, ctrl.is_list, vals_str.join(", ")).unwrap();
    }
    writeln!(f, "];\n").unwrap();

    writeln!(f, "#[allow(dead_code)] pub struct ProfileControlOverride {{ pub control_id: &'static str, pub address_override: Option<u8>, pub values: &'static [(u16, &'static str)] }}").unwrap();
    writeln!(f, "#[allow(dead_code)] pub struct MonitorProfile {{ pub pnp_name: &'static str, pub display_name: &'static str, pub overrides: &'static [ProfileControlOverride], pub includes: &'static [&'static str] }}").unwrap();
    writeln!(f, "pub const MONITOR_PROFILES: &[MonitorProfile] = &[").unwrap();
    for prof in &profiles {
        let overrides_str: Vec<String> = prof.overrides.iter().map(|o| {
            let vals_str: Vec<String> = o.vals.iter().map(|(v, n)| format!("(0x{:04X}, \"{}\")", v, n.replace("\"", "\\\""))).collect();
            format!("ProfileControlOverride {{ control_id: \"{}\", address_override: {}, values: &[{}] }}", o.id.replace("\"", "\\\""), match o.addr { Some(a) => format!("Some(0x{:02X})", a), None => "None".to_string() }, vals_str.join(", "))
        }).collect();
        let includes_str: Vec<String> = prof
            .includes
            .iter()
            .map(|s| format!("\"{}\"", s.replace("\"", "\\\"")))
            .collect();
        writeln!(f, "    MonitorProfile {{ pnp_name: \"{}\", display_name: \"{}\", overrides: &[{}], includes: &[{}] }},", prof.pnp.replace("\"", "\\\""), prof.name.replace("\"", "\\\""), overrides_str.join(",\n        "), includes_str.join(", ")).unwrap();
    }
    writeln!(f, "];\n").unwrap();
}
