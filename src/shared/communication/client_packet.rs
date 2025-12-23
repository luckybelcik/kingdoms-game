use crate::{
    client::client::client_actions::PlayerActions, shared::communication::player_id::PlayerId,
};

pub struct ClientPacket {
    pub player_id: PlayerId,
    pub action: ClientAction,
}

pub enum ClientAction {
    Ping,
    RequestPlayerData,
    PlayerAction(PlayerActions),
    DebugPlayer,
    DebugChunks,
}
