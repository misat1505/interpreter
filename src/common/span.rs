use crate::common::position::Position;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Span {
    start: Position,
    end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Span {
        Span { start, end }
    }

    pub fn start(&self) -> Position {
        self.start
    }

    pub fn end(&self) -> Position {
        self.end
    }

    pub fn join(start: Span, end: Span) -> Span {
        Span::new(start.start(), end.end())
    }
}
