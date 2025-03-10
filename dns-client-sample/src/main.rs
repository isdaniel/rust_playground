use std::env;
use std::net::ToSocketAddrs;

fn main() {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    // Ensure an argument is passed (the FQDN)
    if args.len() < 2 {
        eprintln!("Usage: dns_lookup <FQDN>");
        std::process::exit(1);
    }

    // The FQDN passed as a command-line argument
    let fqdn = &args[1];

    // Append port 0 since ToSocketAddrs requires it
    let fqdn_with_port = format!("{}:0", fqdn);

    // Perform DNS lookup using ToSocketAddrs
    match fqdn_with_port.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                println!("Resolved IP address: {}", addr.ip());
            }
        }
        Err(e) => {
            eprintln!("Failed to lookup {}: {}", fqdn, e);
        }
    }
}
