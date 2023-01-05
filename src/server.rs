use std::{
    io::BufReader,
    net::{TcpListener, TcpStream},
    thread,
};

pub struct Endpoint {
    pub interface: String,
    pub port: u32,
    pub num_acceptors: u32,
}

impl Endpoint {
    pub fn start(&self) {
        let interface = &self.interface;
        let port = &self.port;
        let addr = format!("{interface}:{port}");
        let listener = TcpListener::bind(addr).unwrap();

        if self.num_acceptors > 1 {
            for _ in 1..self.num_acceptors {
                let clone = listener.try_clone().unwrap();
                thread::spawn(move || accept_loop(clone));
            }
        }

        accept_loop(listener);
    }
}
fn accept_loop(listener: TcpListener) {
    loop {
        match listener.accept() {
            Ok((stream, _addr)) => handle_connection(stream),
            Err(e) => println!("couldn't get client: {e:?}"),
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // Pass buf_reader to parse here

    // Handle parsed request here

    // For now just print
    println!("Received request: {buf_reader:#?}");
}
