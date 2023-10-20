// This code is created with my fullest frustration <3

use std::any::Any;

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
