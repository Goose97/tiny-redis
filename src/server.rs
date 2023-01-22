use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    sync::mpsc::{Receiver, Sender},
    thread,
};

use std::time::Instant;

use crate::connection::{inbound, outbound};
use crate::core::{Command, Core};
use crate::job_queue::deque::Queue;
use crate::job_queue::JobQueue;

pub struct Server {
    pub interface: String,
    pub port: usize,
}

#[derive(Clone)]
pub struct CommandWithSender(Command, Sender<Vec<u8>>);

impl Server {
    pub fn start(&self) {
        let interface = &self.interface;
        let port = &self.port;
        let addr = format!("{interface}:{port}");
        let listener = TcpListener::bind(addr).unwrap();

        let mut core = Core::new();
        let job_queue: Queue<CommandWithSender> = Queue::new();

        // Acceptor thread
        let job_queue_clone = job_queue.clone();
        thread::spawn(move || accept_loop(listener, job_queue_clone));

        // Main thread
        loop {
            let CommandWithSender(command, sender) = job_queue.dequeue();
            let response = core.handle_command(command);
            let response_bytes = outbound::encode(response);
            sender.send(response_bytes).unwrap();
        }
    }
}

fn accept_loop(listener: TcpListener, job_queue: Queue<CommandWithSender>) {
    loop {
        match listener.accept() {
            Ok((stream, _addr)) => {
                let cloned_queue = job_queue.clone();
                thread::spawn(move || handle_connection(stream, cloned_queue));
            }
            Err(e) => println!("couldn't get client: {e:?}"),
        }
    }
}

fn handle_connection(stream: TcpStream, job_queue: Queue<CommandWithSender>) {
    let mut cloned_stream = stream.try_clone().unwrap();
    let command_iter = inbound::CommandIter::new(stream);
    let start = Instant::now();

    for command in command_iter {
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

        let duration = start.elapsed();
        log::debug!("Parse command: {command:?}. Took: {duration:?}");

        let start = Instant::now();
        job_queue.enqueue(CommandWithSender(command, tx));
        let duration = start.elapsed();
        log::debug!("Enqueue took: {duration:?}");

        let start = Instant::now();
        let response = rx.recv().unwrap();
        let duration = start.elapsed();
        log::debug!("Wait for response took: {duration:?}");

        let start = Instant::now();
        cloned_stream
            .write_all(&response)
            .expect("Fail to write to socket");

        let duration = start.elapsed();
        log::debug!("Write response took: {:?}", duration);
    }

    log::debug!("Thread is terminating");
}
