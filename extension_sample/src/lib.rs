use pgrx::prelude::*;
use get_if_addrs::{get_if_addrs,IfAddr};
use std::error::Error;

pgrx::pg_module_magic!();

#[pg_extern]
fn hello_extension_sample() -> &'static str {
    "Hello, extension_sample"
}

#[pg_extern]
fn range(s:i32, e:i32) -> pgrx::Range<i32>{
    (s..e).into()
}


#[pg_extern]
fn get_server_ip() -> String {
    match get_local_ip() {
        Ok(ip) => ip,
        Err(_) => "Failed to get IP address".to_string(),
    }
}

fn get_local_ip() -> Result<String, Box<dyn Error>> {
    let if_addrs = get_if_addrs()?;
    for if_addr in if_addrs {
        if let IfAddr::V4(ref ifv4) = if_addr.addr {
            let ipv4 = ifv4.ip;
            if !ipv4.is_loopback() {
                return Ok(ipv4.to_string());
            }
        }
    }
    Err("No non-loopback IP address found".into())
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_extension_sample() {
        assert_eq!("Hello, extension_sample", crate::hello_extension_sample());
    }

}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
