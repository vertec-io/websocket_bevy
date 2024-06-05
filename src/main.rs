mod client;
mod server;
// mod shared;

#[cfg(feature = "client")]
fn main() {
    client::main();
}

#[cfg(not(feature = "client"))]
fn main() {
    server::main();
}
