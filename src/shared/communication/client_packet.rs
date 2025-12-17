use crate::shared::communication::player_id::PlayerId;

pub struct ClientPacket {
    pub player_id: PlayerId,
    pub action: ClientAction,
}

pub enum ClientAction {
    Ping,
}
