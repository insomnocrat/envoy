use super::utf8::*;
use super::utf8::{CR, CRLF, DANGLING_CHUNK, LF, NULL};
use std::iter::Peekable;
use std::slice::Iter;
use std::vec::IntoIter;

pub trait Buffer {
    fn trim(self) -> Vec<u8>;
    fn trim_null(self) -> Vec<u8>;
    fn trim_end(self) -> Vec<u8>;
    fn read_line(&mut self) -> Option<Vec<u8>>;
    fn read_to_space(&mut self) -> Vec<u8>;
}

impl Buffer for Vec<u8> {
    fn trim(self) -> Vec<u8> {
        self.trim_null().trim_end()
    }
    fn trim_null(self) -> Vec<u8> {
        self.into_iter().filter(|b| *b != NULL).collect()
    }
    fn trim_end(self) -> Vec<u8> {
        let mut trimmed = self;
        while trimmed.ends_with(CRLF) {
            trimmed.pop();
            trimmed.pop();
        }
        if trimmed.ends_with(DANGLING_CHUNK) {
            trimmed.pop();
            trimmed.pop();
            trimmed.pop();
        }

        trimmed
    }
    fn read_line(&mut self) -> Option<Vec<u8>> {
        let mut iter = self.iter().peekable();
        iter.read_line()
    }
    fn read_to_space(&mut self) -> Vec<u8> {
        let mut iter = self.iter().peekable();
        iter.read_to_space()
    }
}

impl Buffer for Peekable<Iter<'_, u8>> {
    fn trim(self) -> Vec<u8> {
        self.trim_null().trim_end()
    }
    fn trim_null(self) -> Vec<u8> {
        self.filter(|b| **b != NULL).map(|b| *b).collect()
    }
    fn trim_end(self) -> Vec<u8> {
        let mut trimmed = self.map(|b| *b).collect::<Vec<u8>>();
        while trimmed.ends_with(CRLF) {
            trimmed.pop();
            trimmed.pop();
        }
        if trimmed.ends_with(DANGLING_CHUNK) {
            trimmed.pop();
            trimmed.pop();
            trimmed.pop();
        }

        trimmed
    }
    fn read_line(&mut self) -> Option<Vec<u8>> {
        let mut recorded = vec![];
        while let Some(byte) = self.next() {
            if *byte == CR {
                if self.next_if(|b| **b == LF).is_some() {
                    break;
                }
            }
            recorded.push(*byte);
        }

        match recorded.is_empty() {
            false => Some(recorded),
            true => None,
        }
    }
    fn read_to_space(&mut self) -> Vec<u8> {
        let mut recorded = vec![];
        while let Some(byte) = self.next() {
            if *byte == SP {
                break;
            }
            recorded.push(*byte);
        }

        recorded
    }
}

impl Buffer for Peekable<IntoIter<u8>> {
    fn trim(self) -> Vec<u8> {
        self.trim_null().trim_end()
    }
    fn trim_null(self) -> Vec<u8> {
        self.filter(|b| *b != NULL).collect()
    }
    fn trim_end(self) -> Vec<u8> {
        let mut trimmed = self.collect::<Vec<u8>>();
        while trimmed.ends_with(CRLF) {
            trimmed.pop();
            trimmed.pop();
        }
        if trimmed.ends_with(DANGLING_CHUNK) {
            trimmed.pop();
            trimmed.pop();
            trimmed.pop();
        }

        trimmed
    }
    fn read_line(&mut self) -> Option<Vec<u8>> {
        let mut recorded = vec![];
        while let Some(byte) = self.next() {
            if byte == CR {
                if self.next_if(|b| *b == LF).is_some() {
                    break;
                }
            }
            recorded.push(byte);
        }

        match recorded.is_empty() {
            false => Some(recorded),
            true => None,
        }
    }
    fn read_to_space(&mut self) -> Vec<u8> {
        let mut recorded = vec![];
        while let Some(byte) = self.next() {
            if byte == SP {
                break;
            }
            recorded.push(byte);
        }

        recorded
    }
}
