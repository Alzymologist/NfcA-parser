#[derive(Debug, Eq, PartialEq)]
pub enum FrameError {
    CrcMismatch,
    EmptyFrame,
    ParityBit,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManchesterError {
    FirstNotD,
    Frame(FrameError),
    IncompleteFrame,
    NoAddingToComplete,
    UnexpectedOddInterval(u16),
    UnexpectedEvenInterval(u16),
}

#[derive(Debug, Eq, PartialEq)]
pub enum MillerError {
    Frame(FrameError),
    IncompleteFrame,
    UnexpectedInterval(u16),
    UnexpectedMillerOffInterval(u16),
    WrongMillerSequence,
}
