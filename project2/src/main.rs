use clap::{arg, command};
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const T_GOSSIP: u64 = 2;
const T_FAIL: u64 = 10;
const T_CLEANUP: u64 = 20;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    msg_id: usize,
    receiver_id: usize,
    heartbeats: HashMap<usize, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum State {
    Running,
    Faulty,
    Removed,
}

#[derive(Debug, Clone)]
struct Node {
    state: State,
    updated: Instant,
    heartbeat: u64,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            state: State::Running,
            updated: Instant::now(),
            heartbeat: 0,
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

    let mut nodes = HashMap::new();
    for i in 0..num {
        nodes.insert(i, Node::default());
    }
    let nodes = Arc::new(Mutex::new(nodes));

    // subscriber thread
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
                subscriber.set_rcvtimeo(100).unwrap();
                subscriber.set_subscribe(b"").expect("failed subscribing");
                subscribers.insert(i, subscriber);
            }
        }
        loop {
            for (i, subscriber) in subscribers.iter() {
                if let Ok(Ok(envelope)) = subscriber.recv_string(0) {
                    let message = subscriber.recv_string(0).unwrap().unwrap();
                    let message: Message = serde_json::from_str(&message).unwrap();

                    if envelope.eq("GOSSIP") & (message.receiver_id == id) {
                        log(
                            id,
                            format!(
                                "Recv msg_id {} heartbeats {:?} received from node {i}",
                                message.msg_id, message.heartbeats
                            )
                            .as_str(),
                        );

                        // update timestamp and reset state
                        thread_nodes.lock().unwrap().get_mut(i).unwrap().updated = Instant::now();
                        thread_nodes.lock().unwrap().get_mut(i).unwrap().state = State::Running;

                        // update heartbeats with received ones
                        let mut nodes = thread_nodes.lock().unwrap();
                        message.heartbeats.into_iter().for_each(|(i, received)| {
                            let node = nodes.get_mut(&i).unwrap();
                            node.heartbeat = (node.heartbeat).max(received);
                        });
                    }
                } else {
                    continue;
                }
            }
        }
    });

    // t_fail check thread
    let thread_nodes = Arc::clone(&nodes);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        for (i, node) in thread_nodes
            .lock()
            .unwrap()
            .iter_mut()
            .filter(|(i, _)| !(*i).eq(&id))
        {
            if matches!(node.state, State::Running)
                & (node.updated.elapsed() > Duration::from_secs(T_FAIL))
            {
                log(id, format!("Node {i} set to faulty").as_str());
                node.state = State::Faulty;
                node.updated = Instant::now();
            } else if (matches!(node.state, State::Faulty))
                & (node.updated.elapsed() > Duration::from_secs(T_CLEANUP))
            {
                log(id, format!("Node {i} removed from neighbor list").as_str());
                node.state = State::Removed;
            }
        }
    });

    // publisher
    let mut rng = rand::thread_rng();
    let mut msg_id = 0;
    loop {
        let receiver_id = nodes
            .lock()
            .unwrap()
            .iter()
            .filter(|(i, node)| !(*i).eq(&id) & !matches!(node.state, State::Removed))
            .map(|(i, _)| i)
            .cloned()
            .choose(&mut rng);

        if let Some(receiver_id) = receiver_id {
            nodes.lock().unwrap().get_mut(&id).unwrap().heartbeat += 1;
            let heartbeats: HashMap<usize, u64> = nodes
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, node)| !matches!(node.state, State::Removed))
                .map(|(i, node)| (*i, node.heartbeat))
                .collect();
            log(
                id,
                format!(
                    "Send msg_id {msg_id} heartbeats {:?} to node {receiver_id}",
                    heartbeats
                )
                .as_str(),
            );
            publisher
                .send("GOSSIP", zmq::SNDMORE)
                .expect("failed sending second envelope");
            let message = Message {
                msg_id,
                receiver_id,
                heartbeats,
            };
            publisher.send(serde_json::to_string(&message).unwrap().as_str(), 0)?;
            msg_id += 1;
            thread::sleep(Duration::from_secs(T_GOSSIP));
        } else {
            log(id, "No nodes in neighbor list");
            thread::sleep(Duration::from_secs(T_GOSSIP));
            continue;
        }

        // if rng.gen_range(0..30) < 1 {
        //     break;
        // }
    }
    // log(id, "Terminating ...");
    // Ok(())
}
