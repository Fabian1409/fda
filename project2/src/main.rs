use clap::{arg, command};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    sender_id: usize,
    receiver_id: usize,
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

    println!("Node {id} starting");

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();
    publisher
        .bind(format!("tcp://*:{}", 5550 + id).as_str())
        .expect("failed binding publisher");

    thread::spawn(move || {
        let context = zmq::Context::new();
        let mut subs = Vec::new();
        for i in 0..num {
            let subscriber = context.socket(zmq::SUB).unwrap();
            subscriber
                .connect(format!("tcp://localhost:{}", 5550 + i).as_str())
                .expect("failed connecting subscriber");
            subscriber.set_subscribe(b"").expect("failed subscribing");
            subs.push(subscriber);
        }
        loop {
            for (i, subscriber) in subs.iter().enumerate() {
                if i == id {
                    continue;
                }
                let envelope = subscriber
                    .recv_string(0)
                    .expect("failed receiving envelope")
                    .unwrap();
                let message = subscriber
                    .recv_string(0)
                    .expect("failed receiving message")
                    .unwrap();
                let status: Status = serde_json::from_str(&message).unwrap();

                if envelope.eq("GOSSIP") {
                    println!("GOSSIP msg received by {id} from {}", status.sender_id);
                }
            }
        }
    });

    let mut rng = rand::thread_rng();
    loop {
        let mut receiver_id = id;
        while receiver_id == id {
            receiver_id = rng.gen_range(0..num);
        }
        println!("GOSSIP msg sent by {id} to {receiver_id}");
        publisher
            .send("GOSSIP", zmq::SNDMORE)
            .expect("failed sending second envelope");
        let status = Status {
            sender_id: id,
            receiver_id,
        };
        publisher.send(serde_json::to_string(&status).unwrap().as_str(), 0)?;

        thread::sleep(Duration::from_secs(2));

        if rng.gen_range(0..3) < 1 {
            break;
        }
    }
    println!("Terminating {id} ...");
    Ok(())
}
