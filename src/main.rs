mod server;

fn main() {
    let endpoint = server::Endpoint {
        interface: String::from("127.0.0.1"),
        port: 7878,
        num_acceptors: 4,
    };

    endpoint.start();
}