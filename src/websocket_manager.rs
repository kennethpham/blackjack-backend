use std::{collections::HashMap, net::SocketAddr, ops::ControlFlow};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream::SplitSink, SinkExt};
use mongodb::bson::Uuid;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

#[derive(Debug, Deserialize, Serialize)]
pub struct SendWS {
    pub msg_type: String,
    pub msg_data: String,
}

#[derive(Debug)]
pub enum Command {
    AddWS {
        ws_send: SplitSink<WebSocket, Message>,
        resp: oneshot::Sender<Uuid>,
    },
    DeleteWS {
        id: Uuid,
    },
    SendWS {
        id: Uuid,
        msg: SendWS,
    },
}

pub struct WebSocketManager {
    ws_map: HashMap<Uuid, SplitSink<WebSocket, Message>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        WebSocketManager {
            ws_map: HashMap::new(),
        }
    }

    pub fn add_ws(&mut self, id: Uuid, sender: SplitSink<WebSocket, Message>) {
        match self.ws_map.insert(id.clone(), sender) {
            None => println!("add new {:?}", self.ws_map[&id.clone()]),
            Some(old) => println!("replaced old: {:?} with new: {:?}", old, self.ws_map[&id]),
        }
    }

    pub async fn update_all(&mut self, id: Option<Uuid>) {
        for (key, val) in &mut self.ws_map {
            match id {
                Some(id) => {
                    if key.eq(&id) {
                        continue;
                    }
                }
                None => (),
            }
            let msg = SendWS {
                msg_type: "new ws added".to_string(),
                msg_data: id.unwrap().clone().to_string(),
            };
            let _ = val
                .send(Message::Text(serde_json::ser::to_string(&msg).unwrap()))
                .await;
        }
    }

    pub fn get_ws(&self, id: Uuid) -> Option<&SplitSink<WebSocket, Message>> {
        self.ws_map.get(&id)
    }

    pub async fn send_msg(&mut self, id: Uuid, msg: SendWS) {
        let ws_send = self.ws_map.get_mut(&id).unwrap();
        let _ = ws_send
            .send(Message::Text(serde_json::ser::to_string(&msg).unwrap()))
            .await;
    }

    pub fn remove_ws(&mut self, id: Uuid) -> Option<SplitSink<WebSocket, Message>> {
        self.ws_map.remove(&id)
    }
}
