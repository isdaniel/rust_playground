use std::io::{self, BufRead, Write};
use std::net::SocketAddr;

use raft_core::rpc::*;
use raft_core::transport;
use raft_kv::kv::{KvCommand, KvQuery, KvResult};

/// A simple interactive CLI client for the Raft K/V store.
///
/// Usage:
///   raft-client <server_addr>
///
/// Commands:
///   get <key>
///   set <key> <value>
///   delete <key>
///   quit
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: raft-client <server_addr>  (e.g. 127.0.0.1:9001)");
        std::process::exit(1);
    }

    let mut addr: SocketAddr = args[1].parse().unwrap_or_else(|e| {
        eprintln!("invalid address '{}': {}", args[1], e);
        std::process::exit(1);
    });

    println!("connected to raft cluster via {}", addr);
    println!("commands: get <key> | set <key> <value> | delete <key> | quit");

    let stdin = io::stdin();
    let mut request_id: u64 = 0;

    print!("> ");
    io::stdout().flush().ok();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let parts: Vec<&str> = line.trim().splitn(3, ' ').collect();
        if parts.is_empty() || parts[0].is_empty() {
            print!("> ");
            io::stdout().flush().ok();
            continue;
        }

        let client_cmd = match parts[0].to_lowercase().as_str() {
            "quit" | "exit" => break,
            "get" => {
                if parts.len() < 2 {
                    println!("usage: get <key>");
                    print!("> ");
                    io::stdout().flush().ok();
                    continue;
                }
                let query = KvQuery::Get {
                    key: parts[1].to_string(),
                };
                ClientCommand::Query {
                    payload: serde_json::to_vec(&query).unwrap(),
                }
            }
            "set" => {
                if parts.len() < 3 {
                    println!("usage: set <key> <value>");
                    print!("> ");
                    io::stdout().flush().ok();
                    continue;
                }
                let cmd = KvCommand::Set {
                    key: parts[1].to_string(),
                    value: parts[2].to_string(),
                };
                ClientCommand::Mutate {
                    payload: serde_json::to_vec(&cmd).unwrap(),
                }
            }
            "delete" | "del" => {
                if parts.len() < 2 {
                    println!("usage: delete <key>");
                    print!("> ");
                    io::stdout().flush().ok();
                    continue;
                }
                let cmd = KvCommand::Delete {
                    key: parts[1].to_string(),
                };
                ClientCommand::Mutate {
                    payload: serde_json::to_vec(&cmd).unwrap(),
                }
            }
            other => {
                println!("unknown command: {}", other);
                print!("> ");
                io::stdout().flush().ok();
                continue;
            }
        };

        request_id += 1;
        let req = RpcMessage::ClientRequest(ClientRequest {
            request_id,
            command: client_cmd,
        });

        match send_request(&req, &mut addr).await {
            Ok(resp) => print_response(&resp),
            Err(e) => println!("error: {}", e),
        }

        print!("> ");
        io::stdout().flush().ok();
    }

    println!("bye!");
}

/// Send a request, following NotLeader redirects automatically.
async fn send_request(
    req: &RpcMessage,
    addr: &mut SocketAddr,
) -> Result<ClientResponse, String> {
    let max_redirects = 5;
    let mut current_addr = *addr;

    for _ in 0..max_redirects {
        match transport::rpc_call(current_addr, req).await {
            Ok(RpcMessage::ClientResponse(resp)) => match &resp.result {
                ClientResult::NotLeader {
                    leader_addr: Some(la),
                } => {
                    println!("(redirecting to leader at {})", la);
                    current_addr = la.parse().map_err(|e| format!("bad addr: {}", e))?;
                    *addr = current_addr; // remember for next request
                    continue;
                }
                _ => return Ok(resp),
            },
            Ok(other) => return Err(format!("unexpected response: {:?}", other)),
            Err(e) => return Err(format!("{}", e)),
        }
    }
    Err("too many redirects".into())
}

fn print_response(resp: &ClientResponse) {
    match &resp.result {
        ClientResult::Ok { value: Some(v) } => {
            // Deserialize KvResult
            match serde_json::from_slice::<KvResult>(v) {
                Ok(KvResult::Value(Some(s))) => println!("{}", s),
                Ok(KvResult::Value(None)) => println!("(nil)"),
                Err(_) => {
                    // Fallback: print raw bytes as string
                    println!("{}", String::from_utf8_lossy(v));
                }
            }
        }
        ClientResult::Ok { value: None } => println!("(nil)"),
        ClientResult::Error { message } => println!("ERROR: {}", message),
        ClientResult::NotLeader { leader_addr } => {
            println!("NOT LEADER (leader: {:?})", leader_addr)
        }
    }
}
