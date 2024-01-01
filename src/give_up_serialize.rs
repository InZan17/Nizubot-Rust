// This code is created with my fullest frustration <3

use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use crate::managers::{
    cotd_manager::{ColorInfo, CotdRoleData},
    currency_manager::CurrenciesInfo,
    detector_manager::DetectorInfo,
    remind_manager::RemindInfo,
};

pub trait GiveUpSerialize: Any {
    fn serialize_json(&self) -> String;
}

impl GiveUpSerialize for String {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for i32 {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for HashMap<u64, ColorInfo> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for Vec<u64> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for HashSet<u64> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for CotdRoleData {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for Vec<RemindInfo> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for Vec<DetectorInfo> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for HashMap<String, u64> {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl GiveUpSerialize for CurrenciesInfo {
    fn serialize_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
