use codec::{Decode, Encode};

pub type ParaId = u32;
pub type Timestamp = u64;

#[derive(Encode, Decode)]
pub struct TimestampedMsg {
    pub at: Timestamp,
    pub payload: Vec<u8>,
}

#[derive(Decode, Encode)]
pub struct TargetedMsg {
    pub recepient: ParaId,
    pub msg: TimestampedMsg,
}

#[derive(Encode, Decode)]
pub enum Req {
    /// Enqueue a message.
    Enqueue { recepient: ParaId, payload: Vec<u8> },
    /// Receive all timestamped messages.
    Poll,
    /// Send all enqueued messages.
    FanOut,
}

#[derive(Encode, Decode)]
pub struct Resp {
    // (sender, msg)
    pub inbound: Vec<(ParaId, Vec<TimestampedMsg>)>,
}
