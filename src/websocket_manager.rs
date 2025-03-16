use std::{collections::HashMap, net::SocketAddr, ops::ControlFlow};

use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream::SplitSink, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{AppState, DB};

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
    pub msg_data_arr: Option<Vec<String>>,
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
    UpdateUserList { },
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

    pub async fn update_all_list(&mut self) {
        let uuid_vec = self.get_all_uuids();
        for (_, val) in &mut self.ws_map {
            let msg = SendWS {
                msg_type: MsgType::UpdateUserList,
                msg_data_str: None,
                msg_data_arr: Some(Vec::from_iter(uuid_vec.iter().map(|x| x.to_string()))),

            };
            let _ = val
                .send(Message::Text(serde_json::ser::to_string(&msg).unwrap()))
                .await;
        }
    }

    pub fn get_ws(&self, id: Uuid) -> Option<&SplitSink<WebSocket, Message>> {
        self.ws_map.get(&id)
    }

    pub fn get_all_uuids(&self) -> Vec<Uuid> {
        Vec::from_iter(self.ws_map.keys().map(|x| x.clone()))
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
