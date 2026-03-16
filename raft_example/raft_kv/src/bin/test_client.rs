/// Non-interactive CLI client for scripted testing.
///
/// Usage:
///   raft-test-client <addr> get <key>
///   raft-test-client <addr> set <key> <value>
///   raft-test-client <addr> delete <key>
///
/// Environment:
///   NO_REDIRECT=1  -- disable leader redirect following
///
/// Exit codes:
///   0 = success (value printed to stdout)
///   1 = error (message on stderr)
///   2 = not-leader (leader addr on stdout if known)
use std::net::SocketAddr;
use std::process;

use raft_core::rpc::*;
use raft_core::transport;
use raft_kv::kv::{KvCommand, KvQuery, KvResult};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: raft-test-client <addr> get|set|delete <key> [value]");
        process::exit(1);
    }

    let no_redirect = std::env::var("NO_REDIRECT").unwrap_or_default() == "1";

    let mut addr: SocketAddr = args[1].parse().unwrap_or_else(|e| {
        eprintln!("invalid address '{}': {}", args[1], e);
        process::exit(1);
    });

    let cmd = match args[2].to_lowercase().as_str() {
        "get" => {
            if args.len() < 4 {
                eprintln!("Usage: raft-test-client <addr> get <key>");
                process::exit(1);
            }
            let query = KvQuery::Get {
                key: args[3].clone(),
            };
            ClientCommand::Query {
                payload: serde_json::to_vec(&query).unwrap(),
            }
        }
        "set" => {
            if args.len() < 5 {
                eprintln!("Usage: raft-test-client <addr> set <key> <value>");
                process::exit(1);
            }
            let kv_cmd = KvCommand::Set {
                key: args[3].clone(),
                value: args[4..].join(" "),
            };
            ClientCommand::Mutate {
                payload: serde_json::to_vec(&kv_cmd).unwrap(),
            }
        }
        "delete" | "del" => {
            if args.len() < 4 {
                eprintln!("Usage: raft-test-client <addr> delete <key>");
                process::exit(1);
            }
            let kv_cmd = KvCommand::Delete {
                key: args[3].clone(),
            };
            ClientCommand::Mutate {
                payload: serde_json::to_vec(&kv_cmd).unwrap(),
            }
        }
        other => {
            eprintln!("unknown command: {}", other);
            process::exit(1);
        }
    };

    let req = RpcMessage::ClientRequest(ClientRequest {
        request_id: 1,
        command: cmd,
    });

    let result = if no_redirect {
        send_single(&req, addr).await
    } else {
        send_with_redirect(&req, &mut addr).await
    };

    match result {
        Ok(resp) => match resp.result {
            ClientResult::Ok { value: Some(v) } => {
                match serde_json::from_slice::<KvResult>(&v) {
                    Ok(KvResult::Value(Some(s))) => println!("{}", s),
                    Ok(KvResult::Value(None)) => println!("(nil)"),
                    Err(_) => println!("{}", String::from_utf8_lossy(&v)),
                }
                process::exit(0);
            }
            ClientResult::Ok { value: None } => {
                println!("(nil)");
                process::exit(0);
            }
            ClientResult::Error { message } => {
                eprintln!("ERROR: {}", message);
                process::exit(1);
            }
            ClientResult::NotLeader { leader_addr } => {
                if let Some(la) = leader_addr {
                    println!("NOT_LEADER:{}", la);
                } else {
                    println!("NOT_LEADER:unknown");
                }
                process::exit(2);
            }
        },
        Err(e) => {
            eprintln!("CONNECTION_ERROR: {}", e);
            process::exit(1);
        }
    }
}

/// Send a single request without following redirects.
async fn send_single(req: &RpcMessage, addr: SocketAddr) -> Result<ClientResponse, String> {
    match transport::rpc_call(addr, req).await {
        Ok(RpcMessage::ClientResponse(resp)) => Ok(resp),
        Ok(other) => Err(format!("unexpected response: {:?}", other)),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Send a request, following NotLeader redirects automatically.
async fn send_with_redirect(
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
                    current_addr = la.parse().map_err(|e| format!("bad addr: {}", e))?;
                    *addr = current_addr;
                    continue;
                }
                _ => return Ok(resp),
            },
            Ok(other) => Err(format!("unexpected response: {:?}", other))?,
            Err(e) => return Err(format!("{}", e)),
        }
    }
    Err("too many redirects".into())
}
