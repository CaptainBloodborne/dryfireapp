#![allow(unused)]

use chrono::{DateTime, Utc};

pub enum WeaponClass {}

pub struct Gun {
    manufacturer: String,
    model: String,
    serial_id: String,
    class: WeaponClass,
    caliber: String,
    date_of_purchase: DateTime<Utc>,
    photo_url: Option<String>,
}

impl Gun {
    pub fn new(
        manufacturer: String,
        model: String,
        serial_id: String,
        class: WeaponClass,
        caliber: String,
        date_of_purchase: DateTime<Utc>,
        photo_url: Option<String>,
    ) -> Self {
        Self {
            manufacturer,
            model,
            serial_id,
            class,
            caliber,
            date_of_purchase,
            photo_url,
        }
    }
}

pub struct Round {
    manufacturer: String,
    caliber: String,
    total: u64,
    charge: u64,
}