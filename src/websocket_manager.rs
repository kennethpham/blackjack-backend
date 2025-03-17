use std::{collections::HashMap, net::SocketAddr, ops::ControlFlow};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream::SplitSink, SinkExt};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub enum MsgType {
    UserAdded,
    Data,
    SelfUuid,
    UpdateUserList,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendWS {
    pub msg_type: MsgType,
    pub msg_data_str: Option<String>,
    pub msg_data_keys: Option<Vec<(String, Uuid)>>,
    pub msg_data_arr: Option<String>,
}

#[derive(Debug)]
pub enum Command {
    AddWS {
        ws_send: SplitSink<WebSocket, Message>,
        resp: oneshot::Sender<Uuid>,
        key: (String, Uuid),
    },
    DeleteWS {
        key: (String, Uuid),
    },
    SendWS {
        key: (String, Uuid),
        msg: SendWS,
    },
    UpdateUserList {},
}

pub struct WebSocketManager {
    ws_map: HashMap<(String, Uuid), SplitSink<WebSocket, Message>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        WebSocketManager {
            ws_map: HashMap::new(),
        }
    }

    pub fn add_ws(
        &mut self,
        key: (String, Uuid),
        sender: SplitSink<WebSocket, Message>,
    ) -> (StatusCode, String) {
        match self.ws_map.insert(key.clone(), sender) {
            None => {
                println!("add new {:?}", self.ws_map[&key]);
                (StatusCode::OK, "ws was added".to_string())
            }
            Some(old) => {
                println!("replaced old: {:?} with new: {:?}", old, self.ws_map[&key]);
                (
                    StatusCode::CONFLICT,
                    "ws was not added due to conflicting keys".to_string(),
                )
            }
        }
    }

    pub async fn update_all_list(&mut self) {
        let uuid_vec = self.get_all_uuids();
        for (_, val) in &mut self.ws_map {
            let msg = SendWS {
                msg_type: MsgType::UpdateUserList,
                msg_data_str: None,
                msg_data_keys: Some(Vec::from_iter(uuid_vec.iter().map(|t| t.clone()))),
                msg_data_arr: None,
            };
            let _ = val
                .send(Message::Text(serde_json::ser::to_string(&msg).unwrap()))
                .await;
        }
    }

    pub fn get_ws(&self, id: Uuid, name: String) -> Option<&SplitSink<WebSocket, Message>> {
        self.ws_map.get(&(name, id))
    }

    pub fn get_all_uuids(&self) -> Vec<(String, Uuid)> {
        Vec::from_iter(self.ws_map.keys().map(|x| x.clone()))
    }

    pub async fn send_msg(&mut self, key: (String, Uuid), msg: SendWS) {
        let ws_send = self.ws_map.get_mut(&key).unwrap();
        let _ = ws_send
            .send(Message::Text(serde_json::ser::to_string(&msg).unwrap()))
            .await;
    }

    pub fn remove_ws(&mut self, key: (String, Uuid)) -> Option<SplitSink<WebSocket, Message>> {
        self.ws_map.remove(&key)
    }
}
