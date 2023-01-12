use tiny_redis::server;

fn main() {
    // While researching this topic, we found an interesting performance problem from Linux
    // kernel in the past. This issue is called Thundering herd.
    // It happens when multiple threads are waiting on accept() call on the same socket.
    // When the new connection arrives, every threads will be woke up, but only one thread
    // can grab the connection. This wastes CPU cycles and leads to performance downgrade in
    // high-load services.
    //
    // Some interesting links:
    // https://uwsgi-docs.readthedocs.io/en/latest/articles/SerializingAccept.html
    // http://www.citi.umich.edu/projects/linux-scalability/reports/accept.html
    let endpoint = server::Endpoint {
        interface: String::from("127.0.0.1"),
        port: 7878,
        num_acceptors: 4,
    };

    endpoint.start();
}
