use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Raw record from CSV ingestion
#[derive(Debug, Deserialize)]
pub struct CsvRecord {
    pub carrier_mode: String,
    pub actual_ship: String,
    pub actual_delivery: String,
    pub carrier_posted_service_days: Option<f64>,
    pub customer_distance: Option<f64>,
    pub truckload_service_days: Option<f64>,
    pub all_modes_goal_transit_days: i32,
    pub actual_transit_days: i32,
    pub otd_designation: String,
    pub load_id_pseudo: String,
    pub carrier_pseudo: String,
    pub origin_zip_3d: String,
    pub dest_zip_3d: String,
    pub ship_dow: i32,
    pub ship_week: i32,
    pub ship_month: i32,
    pub ship_year: i32,
    pub lane_zip3_pair: String,
    pub lane_id: String,
    pub distance_bucket: String,
}

/// Carrier mode enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CarrierMode {
    LTL,
    Truckload,
    TLFlatbed,
    TLDry,
}

impl From<&str> for CarrierMode {
    fn from(s: &str) -> Self {
        match s {
            "LTL" => CarrierMode::LTL,
            "Truckload" => CarrierMode::Truckload,
            "TL Flatbed" => CarrierMode::TLFlatbed,
            "TL Dry" => CarrierMode::TLDry,
            _ => CarrierMode::Truckload, // default
        }
    }
}

/// On-time delivery designation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OtdDesignation {
    Early,
    OnTime,
    Late,
}

impl From<&str> for OtdDesignation {
    fn from(s: &str) -> Self {
        match s {
            "Delivered Early" => OtdDesignation::Early,
            "On Time" => OtdDesignation::OnTime,
            "Late" => OtdDesignation::Late,
            _ => OtdDesignation::OnTime,
        }
    }
}

/// Shipment record for SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shipment {
    pub load_id: String,
    pub carrier_mode: CarrierMode,
    pub actual_ship: NaiveDateTime,
    pub actual_delivery: NaiveDateTime,
    pub carrier_posted_service_days: Option<f64>,
    pub customer_distance: Option<f64>,
    pub truckload_service_days: Option<f64>,
    pub goal_transit_days: i32,
    pub actual_transit_days: i32,
    pub otd: OtdDesignation,
    pub ship_dow: i32,
    pub ship_week: i32,
    pub ship_month: i32,
    pub ship_year: i32,
    pub distance_bucket: String,
}

/// Shipment with ID from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentRecord {
    pub id: Thing,
    #[serde(flatten)]
    pub shipment: Shipment,
}

/// Carrier entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carrier {
    pub carrier_id: String,
}

/// Location (ZIP code region)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub zip3: String,
    pub state: Option<String>,
}

/// Lane connecting two locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lane {
    pub lane_id: String,
    pub zip3_pair: String,
}

impl CsvRecord {
    pub fn to_shipment(&self) -> anyhow::Result<Shipment> {
        let actual_ship = NaiveDateTime::parse_from_str(&self.actual_ship, "%Y-%m-%d %H:%M:%S")?;
        let actual_delivery =
            NaiveDateTime::parse_from_str(&self.actual_delivery, "%Y-%m-%d %H:%M:%S")?;

        Ok(Shipment {
            load_id: self.load_id_pseudo.clone(),
            carrier_mode: CarrierMode::from(self.carrier_mode.as_str()),
            actual_ship,
            actual_delivery,
            carrier_posted_service_days: self.carrier_posted_service_days,
            customer_distance: self.customer_distance,
            truckload_service_days: self.truckload_service_days,
            goal_transit_days: self.all_modes_goal_transit_days,
            actual_transit_days: self.actual_transit_days,
            otd: OtdDesignation::from(self.otd_designation.as_str()),
            ship_dow: self.ship_dow,
            ship_week: self.ship_week,
            ship_month: self.ship_month,
            ship_year: self.ship_year,
            distance_bucket: self.distance_bucket.clone(),
        })
    }
}
