use clap::{arg, command};
use rand::{seq::IteratorRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const T_GOSSIP: u64 = 2;
const T_FAIL: u64 = 5;
const T_CLEANUP: u64 = 10;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    receiver_id: usize,
    heartbeats: HashMap<usize, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum State {
    Running,
    Faulty,
}

#[derive(Debug, Clone)]
struct Node {
    state: State,
    updated: Instant,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            state: State::Running,
            updated: Instant::now(),
        }
    }
}

fn log(id: usize, str: &str) {
    println!("[{id}] {str}");
}

fn main() -> Result<(), zmq::Error> {
    let matches = command!()
        .arg(arg!(<num> "Number of nodes"))
        .arg(arg!(<node> "Node id (0..n)"))
        .get_matches();
    let id: usize = matches
        .get_one::<String>("node")
        .unwrap()
        .parse()
        .expect("invalid node id");
    let num: usize = matches
        .get_one::<String>("num")
        .unwrap()
        .parse()
        .expect("invalid number of nodes");

    log(id, "starting");

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();
    publisher
        .bind(format!("tcp://*:{}", 5550 + id).as_str())
        .expect("failed binding publisher");

    let mut heartbeats = HashMap::new();
    let mut nodes = HashMap::new();
    for i in 0..num {
        heartbeats.insert(i, 0u64);
        if i != id {
            nodes.insert(i, Node::default());
        }
    }
    let heartbeats = Arc::new(Mutex::new(heartbeats));
    let nodes = Arc::new(Mutex::new(nodes));

    // subscriber thread
    let thread_heartbeats = Arc::clone(&heartbeats);
    let thread_nodes = Arc::clone(&nodes);
    thread::spawn(move || {
        let context = zmq::Context::new();
        let mut subscribers = HashMap::new();
        for i in 0..num {
            if i != id {
                let subscriber = context.socket(zmq::SUB).unwrap();
                subscriber
                    .connect(format!("tcp://localhost:{}", 5550 + i).as_str())
                    .expect("failed connecting subscriber");
                subscriber.set_subscribe(b"").expect("failed subscribing");
                subscribers.insert(i, subscriber);
            }
        }
        loop {
            for (i, subscriber) in subscribers.iter() {
                let envelope = subscriber
                    .recv_string(0)
                    .expect("failed receiving envelope")
                    .unwrap();
                let message = subscriber
                    .recv_string(0)
                    .expect("failed receiving message")
                    .unwrap();
                let message: Message = serde_json::from_str(&message).unwrap();

                if envelope.eq("GOSSIP") & (message.receiver_id == id) {
                    log(
                        id,
                        format!(
                            "Received heartbeats {:?} received from node {i}",
                            message.heartbeats
                        )
                        .as_str(),
                    );

                    // update timestamp
                    thread_nodes.lock().unwrap().get_mut(i).unwrap().updated = Instant::now();
                    thread_nodes.lock().unwrap().get_mut(i).unwrap().state = State::Running;

                    // update heartbeats with received ones
                    message.heartbeats.into_iter().for_each(|(node, received)| {
                        match thread_heartbeats.lock().unwrap().get_mut(&node) {
                            Some(heartbeat) => {
                                *heartbeat = (*heartbeat).max(received);
                            }
                            None => {
                                thread_nodes.lock().unwrap().insert(node, Node::default());
                                thread_heartbeats.lock().unwrap().insert(node, received);
                            }
                        }
                    });
                }
            }
        }
    });

    // t_fail check thread
    let thread_heartbeats = Arc::clone(&heartbeats);
    let thread_nodes = Arc::clone(&nodes);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        for (i, node) in thread_nodes.lock().unwrap().iter_mut() {
            if matches!(node.state, State::Running)
                & (node.updated.elapsed() > Duration::from_secs(T_FAIL))
            {
                log(id, format!("Node {i} set to faulty").as_str());
                node.state = State::Faulty;
                node.updated = Instant::now();
            } else if node.updated.elapsed() > Duration::from_secs(T_CLEANUP) {
                log(id, format!("Node {i} removed from neighbor list").as_str());
                thread_heartbeats.lock().unwrap().remove(i);
                thread_nodes.lock().unwrap().remove(i);
            }
        }
    });

    // publisher
    let mut rng = rand::thread_rng();
    loop {
        if heartbeats.lock().unwrap().len() == 1 {
            log(id, "No nodes in neighbor list");
        } else {
            let mut receiver_id = id;
            while receiver_id == id {
                receiver_id = *heartbeats.lock().unwrap().keys().choose(&mut rng).unwrap();
            }
            *heartbeats.lock().unwrap().get_mut(&id).unwrap() += 1;
            log(
                id,
                format!(
                    "Sending heartbeats {:?} to node {receiver_id}",
                    heartbeats.lock().unwrap()
                )
                .as_str(),
            );
            publisher
                .send("GOSSIP", zmq::SNDMORE)
                .expect("failed sending second envelope");
            let message = Message {
                receiver_id,
                heartbeats: heartbeats.lock().unwrap().clone(),
            };
            publisher.send(serde_json::to_string(&message).unwrap().as_str(), 0)?;
        }

        thread::sleep(Duration::from_secs(T_GOSSIP));

        // if rng.gen_range(0..30) < 1 {
        //     break;
        // }
    }
    // log(id, "Terminating ...");
    // Ok(())
}
