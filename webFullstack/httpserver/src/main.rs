use server::Server;

mod server;
mod handler;
mod router;

fn main() {
    Server::new("127.0.0.1:3001").run();
}
