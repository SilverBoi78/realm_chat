use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;
use uuid::Uuid;

use common::{ChatMessage, WsMessage};

const CHANNEL_CAPACITY: usize = 128;

pub struct Hub {
    channels: Mutex<HashMap<Uuid, broadcast::Sender<ChatMessage>>>,
    user_channels: Mutex<HashMap<Uuid, broadcast::Sender<WsMessage>>>,
}

impl Hub {
    pub fn new() -> Self {
        Hub {
            channels: Mutex::new(HashMap::new()),
            user_channels: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self, location_id: Uuid) -> broadcast::Receiver<ChatMessage> {
        let mut map = self.channels.lock().unwrap();
        let tx = map.entry(location_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
            tx
        });
        tx.subscribe()
    }

    pub fn publish(&self, location_id: Uuid, msg: ChatMessage) {
        let map = self.channels.lock().unwrap();
        if let Some(tx) = map.get(&location_id) {
            let _ = tx.send(msg);
        }
    }

    pub fn subscribe_user(&self, user_id: Uuid) -> broadcast::Receiver<WsMessage> {
        let mut map = self.user_channels.lock().unwrap();
        let tx = map.entry(user_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
            tx
        });
        tx.subscribe()
    }

    pub fn notify_user(&self, user_id: Uuid, msg: WsMessage) {
        let map = self.user_channels.lock().unwrap();
        if let Some(tx) = map.get(&user_id) {
            let _ = tx.send(msg);
        }
    }
}
