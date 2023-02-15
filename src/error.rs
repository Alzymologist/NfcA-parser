#[derive(Debug, Eq, PartialEq)]
pub enum FrameError {
    CrcMismatch,
    ParityBit,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManchesterError {
    FirstNotD,
    Frame(FrameError),
    IncompleteFrame,
    NoAddingToComplete,
    UnexpectedOddInterval,
    UnexpectedEvenInterval,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MillerError {
    Frame(FrameError),
    IncompleteFrame,
    UnexpectedInterval,
    UnexpectedMillerOffInterval,
    WrongMillerSequence,
}
