//! Carrier name mapping for realistic display
//! Maps anonymized carrier IDs to realistic carrier names

use std::collections::HashMap;
use std::sync::LazyLock;

/// Carrier name mapping - maps hex IDs to realistic carrier names
pub static CARRIER_NAMES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Major LTL Carriers (high volume)
    m.insert("0e32a59c0c8e", "XPO Logistics");
    m.insert("ae9d1671f54a", "FedEx Freight");
    m.insert("cfd59abc9d4b", "Old Dominion");
    m.insert("d29d021b03f6", "Estes Express");
    m.insert("b8e932b33b01", "Saia LTL");
    m.insert("e6c81f092efd", "ABF Freight");
    m.insert("5797a633da7c", "YRC Worldwide");
    m.insert("dbfc03065eae", "R+L Carriers");
    m.insert("de78ac80b8a6", "Southeastern Freight");
    m.insert("e241c58d2bfc", "AAA Cooper");

    // Mid-size carriers
    m.insert("a77afc74f43b", "Dayton Freight");
    m.insert("54874e5091dc", "Central Transport");
    m.insert("cd870e9d66f4", "Averitt Express");
    m.insert("f3966ed1d22b", "Pitt Ohio");
    m.insert("a1f6e862ef0a", "Holland Regional");
    m.insert("b3e6702bc7d2", "New Penn");
    m.insert("312f43423e8e", "Ward Transport");
    m.insert("2c6be28324ea", "Midwest Motor");
    m.insert("19936bf01cc6", "Oak Harbor");
    m.insert("270007da8c2c", "Dependable Highway");

    // Truckload carriers
    m.insert("020d05ae87ec", "J.B. Hunt");
    m.insert("7fa7f958bd51", "Schneider National");
    m.insert("103fb84c7f5b", "Werner Enterprises");
    m.insert("1859c9911606", "Swift Transportation");
    m.insert("2322a0240573", "Knight-Swift");
    m.insert("1fbbcf35d02b", "Heartland Express");

    // Smaller/Regional carriers
    m.insert("029dda1033ee", "Lakeville Motor");
    m.insert("0878916df59b", "Peninsula Truck");
    m.insert("0e9d290aaec8", "Standard Forwarding");
    m.insert("14180a225452", "Wilson Trucking");
    m.insert("17adef8c2fd8", "Dohrn Transfer");
    m.insert("17be8f523e2a", "Roadrunner Freight");
    m.insert("1a7fc00cd480", "Clear Lane Freight");
    m.insert("1f77b01c5146", "Magnum LTL");

    // Additional carriers (can be extended)
    m.insert("3a7bc9d12ef4", "CrossCountry Freight");
    m.insert("4b8cd0e23fa5", "Interstate Motor");
    m.insert("5c9de1f34ab6", "American Freight");
    m.insert("6d0ef2a45bc7", "National Carriers");
    m.insert("7e1fa3b56cd8", "Prime Inc");
    m.insert("8f2ab4c67de9", "Covenant Transport");
    m.insert("9a3bc5d78efa", "US Xpress");
    m.insert("ab4cd6e89f0b", "Landstar System");
    m.insert("bc5de7f9a01c", "Hub Group");
    m.insert("cd6ef8a0b12d", "Echo Global");

    m
});

/// Get carrier display name, falling back to shortened ID if not mapped
pub fn get_carrier_name(carrier_id: &str) -> String {
    CARRIER_NAMES
        .get(carrier_id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Return shortened version for unmapped carriers
            if carrier_id.len() > 8 {
                format!("Carrier-{}", &carrier_id[..8])
            } else {
                format!("Carrier-{}", carrier_id)
            }
        })
}

/// Get carrier display name with max length (for table formatting)
pub fn get_carrier_name_short(carrier_id: &str, max_len: usize) -> String {
    let name = get_carrier_name(carrier_id);
    if name.len() > max_len {
        format!("{}...", &name[..max_len-3])
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_carrier() {
        assert_eq!(get_carrier_name("0e32a59c0c8e"), "XPO Logistics");
    }

    #[test]
    fn test_unknown_carrier() {
        let name = get_carrier_name("unknown123456");
        assert!(name.starts_with("Carrier-"));
    }
}
