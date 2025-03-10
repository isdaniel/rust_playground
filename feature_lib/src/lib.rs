#[cfg(feature = "feature_a")]
pub fn feature_a_function() -> &'static str {
    "Feature A is enabled."
}

#[cfg(feature = "feature_b")]
pub fn feature_b_function() -> &'static str {
    "Feature B is enabled."
}

pub fn runtime_check() -> &'static str {
    if cfg!(feature = "logging") {
        "Logging is enabled!"
    } else {
        "Logging is disabled!"
    }
}
