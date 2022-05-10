use crate::http::Response;

pub struct Stream {
    pub id: u32,
    pub request: Option<Vec<u8>>,
    pub state: State,
    pub response_headers: Vec<u8>,
    pub response_data: Vec<u8>,
    pub termination_code: u8,
}

impl Stream {
    pub fn new(id: u32, request: Vec<u8>) -> Self {
        Self {
            id,
            request: Some(request),
            state: State::Idle,
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
