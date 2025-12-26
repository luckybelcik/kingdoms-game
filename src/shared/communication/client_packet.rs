use serde::{Deserialize, Serialize};

use crate::{
    client::client::client_actions::PlayerActions,
    shared::communication::{player_data::ClientPlayerData, player_id::PlayerId},
};

#[derive(Serialize, Deserialize)]
pub struct ClientPacket {
    pub player_id: PlayerId,
    pub action: ClientAction,
}

#[derive(Serialize, Deserialize)]
pub enum ClientAction {
    Ping,
    RequestPlayerData,
    PlayerAction(PlayerActions),
    DebugPlayer,
    DebugChunks,
    DebugCheckSync(ClientPlayerData),
}
