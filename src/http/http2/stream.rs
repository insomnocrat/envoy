#[derive(Debug, Clone, PartialEq)]
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

impl Default for Stream {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Open,
    HalfClosed,
    Closed,
}

impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}
