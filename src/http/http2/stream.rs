#[derive(Debug)]
pub struct Stream {
    pub id: u32,
    pub state: State,
    pub response_headers: Vec<u8>,
    pub response_data: Vec<u8>,
    pub termination_code: u8,
}

impl Stream {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            state: State::Open,
            response_headers: Vec::new(),
            response_data: Vec::new(),
            termination_code: 0,
        }
    }
    pub fn is_closed(&self) -> bool {
        self.state == State::Closed
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Open,
    HalfClosed,
    Closed,
}
