use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Service;

const POWER_SUPPLIES: &str = "/sys/class/power_supply";

#[derive(Default)]
pub struct Battery {
    // none on a desktop without a battery
    path: Option<PathBuf>,

    // 0 to 100
    percent: u8,

    charging: bool,

    // plugged in and not charging, usually because it's full
    full: bool,
}

// polled, the kernel only exposes the charge as files
impl Service for Battery {
    fn new() -> Self {
        let mut battery = Self {
            path: find(),
            ..Self::default()
        };

        battery.update();

        battery
    }

    fn interval() -> Duration {
        Duration::from_secs(5)
    }

    fn update(&mut self) -> bool {
        // If the path doesn't exist or is invalid, re-discover the battery
        if self.path.as_ref().map_or(true, |p| !p.exists()) {
            self.path = find();
        }

        let Some(path) = &self.path else {
            let before = (self.percent, self.charging, self.full);
            self.percent = 0;
            self.charging = false;
            self.full = false;
            return (self.percent, self.charging, self.full) != before;
        };

        let before = (self.percent, self.charging, self.full);

        let mut capacity = read(path, "capacity");
        let mut status = read(path, "status");

        if capacity.is_empty() {
            self.path = find();
            if let Some(path) = &self.path {
                capacity = read(path, "capacity");
                status = read(path, "status");
            }
        }

        self.percent = capacity.parse().unwrap_or(0);

        self.charging = status == "Charging";
        self.full = status == "Full" || status == "Not charging";

        (self.percent, self.charging, self.full) != before
    }
}

impl Battery {
    pub fn present(&self) -> bool {
        self.path.is_some()
    }

    pub fn percent(&self) -> u8 {
        self.percent
    }

    pub fn charging(&self) -> bool {
        self.charging
    }

    pub fn full(&self) -> bool {
        self.full
    }
}

// finds the system battery, skipping peripheral devices (mice, keyboards, trackballs)
fn find() -> Option<PathBuf> {
    let entries = fs::read_dir(POWER_SUPPLIES).ok()?;
    let mut candidate = None;

    for entry in entries.flatten() {
        let path = entry.path();

        if read(&path, "type") != "Battery" {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Peripheral devices (like wireless mice/trackballs) report scope = "Device"
        let scope = read(&path, "scope");
        if scope != "Device" {
            // Prioritize primary laptop battery like BAT0, BAT1
            if name_str.starts_with("BAT") {
                return Some(path);
            }
            if candidate.is_none() {
                candidate = Some(path);
            }
        }
    }

    candidate
}

fn read(folder: &Path, name: &str) -> String {
    let text = fs::read_to_string(folder.join(name)).unwrap_or_default();

    String::from(text.trim())
}
