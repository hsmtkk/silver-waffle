use axum::extract::Extension;
use axum::handler::get;
use axum::response::Json;
use axum::{AddExtensionLayer,Router};
use crossbeam_channel::{bounded, select, Sender, Receiver};
use log::info;
use serde_json::{json, Value};
use std::sync::Arc;
use std::thread;

struct Counter {
    count: u64,
    up_receiver: Receiver<()>,
    get_sender: Sender<u64>,
}

impl Counter {
    fn new(up_receiver: Receiver<()>, get_sender: Sender<u64>) -> Counter {
        Counter{count:0, up_receiver, get_sender}
    }

    fn run(&mut self) {
        loop {
            select!{
                recv(self.up_receiver) -> _ => {
                    info!("counter: count up");
                    self.count += 1;
                },
                send(self.get_sender, self.count) -> _ => {
                    info!("counter: get count {}", self.count);
                },
            }    
        }
    }
}

struct Channel {
    up_sender: Sender<()>,
    get_receiver: Receiver<u64>,
}

impl Channel {
    fn new(up_sender: Sender<()>, get_receiver: Receiver<u64>) -> Channel {
        Channel{up_sender, get_receiver}
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let (up_sender, up_receiver) = bounded(0);
    let (get_sender, get_receiver) = bounded(0);
    let mut counter = Counter::new(up_receiver, get_sender);
    thread::spawn(move || counter.run());

    let channel = Arc::new(Channel::new(up_sender, get_receiver));
    let app = Router::new().route("/get", get(get_handler)).route("/up", get(up_handler)).layer(AddExtensionLayer::new(channel));
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
}

async fn get_handler(channel: Extension<Arc<Channel>>) -> Json<Value>{
    let get_receiver = &channel.0.get_receiver;
    let count = get_receiver.recv().unwrap();
    info!("axum: get count {}", count);
    Json(json!({"count": count}))
}

async fn up_handler(channel: Extension<Arc<Channel>>){
    info!("axum: count up");
    let up_sender = &channel.0.up_sender;
    up_sender.send(()).unwrap();
}
