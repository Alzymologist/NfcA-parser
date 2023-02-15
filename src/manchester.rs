use bitvec::prelude::{BitVec, Lsb0};

use crate::error::ManchesterError;
use crate::frame::{CompleteCollector, Frame};
use crate::time_record_both_ways::{EntryTimesBoth, SetTimesBoth};

#[derive(Debug, Eq, PartialEq)]
pub enum ManchesterElement {
    D,
    E,
    F,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManchesterCollector {
    Empty,
    InProgress(BitVec<u8, Lsb0>),
    Complete(CompleteCollector),
}

impl ManchesterCollector {
    pub fn add_element(&mut self, element: &ManchesterElement) -> Result<(), ManchesterError> {
        match self {
            ManchesterCollector::Empty => {
                if let ManchesterElement::D = element {
                    *self = ManchesterCollector::InProgress(BitVec::<u8, Lsb0>::new())
                } else {
                    return Err(ManchesterError::FirstNotD);
                }
            }
            ManchesterCollector::InProgress(set) => match element {
                ManchesterElement::D => {
                    set.push(true);
                    *self = ManchesterCollector::InProgress(set.to_bitvec())
                }
                ManchesterElement::E => {
                    set.push(false);
                    *self = ManchesterCollector::InProgress(set.to_bitvec())
                }
                ManchesterElement::F => {
                    *self = ManchesterCollector::Complete(CompleteCollector {
                        data: set.to_bitvec(),
                    })
                }
            },
            ManchesterCollector::Complete(_) => return Err(ManchesterError::NoAddingToComplete),
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ManchesterElementSet {
    element_set: Vec<ManchesterElement>,
}

impl ManchesterElementSet {
    pub fn new() -> Self {
        Self {
            element_set: Vec::new(),
        }
    }

    fn process_previous_d<const TICK_LEN: u32>(
        &mut self,
        time_both: EntryTimesBoth,
    ) -> Result<(), ManchesterError> {
        if (time_both.first_len >= 3 * TICK_LEN) & (time_both.first_len <= 5 * TICK_LEN) {
        } else if (time_both.first_len >= 7 * TICK_LEN) & (time_both.first_len <= 9 * TICK_LEN) {
            self.element_set.push(ManchesterElement::E)
        } else if (time_both.first_len >= 11 * TICK_LEN) & (time_both.first_len <= 13 * TICK_LEN) {
            self.element_set.push(ManchesterElement::F)
        } else {
            return Err(ManchesterError::UnexpectedOddInterval);
        }
        if let Some(second_len) = time_both.second_len {
            match self
                .element_set
                .last()
                .expect("there definitely is an element in the sequence already")
            {
                ManchesterElement::D => {
                    if (second_len >= 3 * TICK_LEN) & (second_len <= 5 * TICK_LEN) {
                        self.element_set.push(ManchesterElement::D)
                    } else {
                        return Err(ManchesterError::UnexpectedEvenInterval);
                    }
                }
                ManchesterElement::E => {
                    if (second_len >= 3 * TICK_LEN) & (second_len <= 5 * TICK_LEN) {
                    } else if (second_len >= 7 * TICK_LEN) & (second_len <= 9 * TICK_LEN) {
                        self.element_set.push(ManchesterElement::D)
                    } else {
                        return Err(ManchesterError::UnexpectedEvenInterval);
                    }
                }
                ManchesterElement::F => {
                    if (second_len >= 3 * TICK_LEN) & (second_len <= 5 * TICK_LEN) {
                        self.element_set.push(ManchesterElement::D)
                    } else {
                        return Err(ManchesterError::UnexpectedEvenInterval);
                    }
                }
            }
        }
        Ok(())
    }

    fn process_previous_e<const TICK_LEN: u32>(
        &mut self,
        time_both: EntryTimesBoth,
    ) -> Result<(), ManchesterError> {
        if (time_both.first_len >= 3 * TICK_LEN) & (time_both.first_len <= 5 * TICK_LEN) {
            self.element_set.push(ManchesterElement::E)
        } else if (time_both.first_len >= 7 * TICK_LEN) & (time_both.first_len <= 9 * TICK_LEN) {
            self.element_set.push(ManchesterElement::F)
        } else {
            return Err(ManchesterError::UnexpectedOddInterval);
        }
        if let Some(second_len) = time_both.second_len {
            match self
                .element_set
                .last()
                .expect("there definitely is an element in the sequence already")
            {
                ManchesterElement::D => unreachable!(),
                ManchesterElement::E => {
                    if (second_len >= 3 * TICK_LEN) & (second_len <= 5 * TICK_LEN) {
                    } else if (second_len >= 7 * TICK_LEN) & (second_len <= 9 * TICK_LEN) {
                        self.element_set.push(ManchesterElement::D)
                    } else {
                        return Err(ManchesterError::UnexpectedEvenInterval);
                    }
                }
                ManchesterElement::F => {
                    if (second_len >= 3 * TICK_LEN) & (second_len <= 5 * TICK_LEN) {
                        self.element_set.push(ManchesterElement::D)
                    } else {
                        return Err(ManchesterError::UnexpectedEvenInterval);
                    }
                }
            }
        }
        Ok(())
    }

    /// Modulation is always present on the PICC. modulation is suppressed for
    /// the duration of F element to indicate the end of frame.
    /// Outer long time intervals are modulated.
    pub(crate) fn add_time_both_interval<const TICK_LEN: u32>(
        &mut self,
        time_both: EntryTimesBoth,
    ) -> Result<(), ManchesterError> {
        match self.element_set.last() {
            None => {
                self.element_set.push(ManchesterElement::D);
                self.process_previous_d::<TICK_LEN>(time_both)
            }
            Some(ManchesterElement::D) => self.process_previous_d::<TICK_LEN>(time_both),
            Some(ManchesterElement::E) => self.process_previous_e::<TICK_LEN>(time_both),
            Some(ManchesterElement::F) => unreachable!(),
        }
    }

    pub fn from_times_both<const TICK_LEN: u32>(
        times_both: SetTimesBoth<TICK_LEN>,
    ) -> Result<Self, ManchesterError> {
        times_both.convert_to_manchester()
    }

    pub fn collect_frame(&self) -> Result<Frame, ManchesterError> {
        let mut collector = ManchesterCollector::Empty;
        for element in self.element_set.iter() {
            collector.add_element(element)?;
        }
        if let ManchesterCollector::Complete(complete_collector) = collector {
            complete_collector
                .to_frame()
                .map_err(ManchesterError::Frame)
        } else {
            Err(ManchesterError::IncompleteFrame)
        }
    }
}

impl Default for ManchesterElementSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::bitvec;

    #[test]
    fn manchester_collector_01() {
        let mut collector = ManchesterCollector::Empty;
        assert!(collector.add_element(&ManchesterElement::E).is_err());
        assert!(collector.add_element(&ManchesterElement::F).is_err());
        collector.add_element(&ManchesterElement::D).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(BitVec::<u8, Lsb0>::new())
        );
    }

    #[test]
    fn manchester_collector_02() {
        let mut collector = ManchesterCollector::InProgress(BitVec::<u8, Lsb0>::new());
        collector.add_element(&ManchesterElement::D).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1])
        );
    }

    #[test]
    fn manchester_collector_03() {
        let mut collector = ManchesterCollector::InProgress(BitVec::<u8, Lsb0>::new());
        collector.add_element(&ManchesterElement::E).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0])
        );
    }

    #[test]
    fn manchester_collector_04() {
        let mut collector = ManchesterCollector::InProgress(BitVec::<u8, Lsb0>::new());
        collector.add_element(&ManchesterElement::F).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::Complete(CompleteCollector {
                data: BitVec::<u8, Lsb0>::new()
            })
        );
    }

    #[test]
    fn manchester_collector_05() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&ManchesterElement::D).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0, 1])
        );
    }

    #[test]
    fn manchester_collector_06() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&ManchesterElement::E).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0, 0])
        );
    }

    #[test]
    fn manchester_collector_07() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&ManchesterElement::F).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 0]
            })
        );
    }

    #[test]
    fn manchester_collector_08() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1]);
        collector.add_element(&ManchesterElement::D).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1, 1])
        );
    }

    #[test]
    fn manchester_collector_09() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1]);
        collector.add_element(&ManchesterElement::E).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1, 0])
        );
    }

    #[test]
    fn manchester_collector_10() {
        let mut collector = ManchesterCollector::InProgress(bitvec![u8, Lsb0; 1]);
        collector.add_element(&ManchesterElement::F).unwrap();
        assert_eq!(
            collector,
            ManchesterCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 1]
            })
        );
    }

    #[test]
    fn manchester_sequence_1() {
        let sequence = [
            ManchesterElement::D,
            ManchesterElement::D,
            ManchesterElement::E,
            ManchesterElement::E,
            ManchesterElement::D,
            ManchesterElement::F,
        ];
        let mut collector = ManchesterCollector::Empty;
        for element in sequence {
            collector.add_element(&element).unwrap();
        }
        assert_eq!(
            collector,
            ManchesterCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 1, 0, 0, 1]
            })
        );
    }

    #[test]
    fn manchester_sequence_2() {
        let sequence = [
            ManchesterElement::D,
            ManchesterElement::D,
            ManchesterElement::E,
            ManchesterElement::F,
        ];
        let mut collector = ManchesterCollector::Empty;
        for element in sequence {
            collector.add_element(&element).unwrap();
        }
        assert_eq!(
            collector,
            ManchesterCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 1, 0]
            })
        );
    }

    #[test]
    fn manchester_time_both_1() {
        let times_set = [
            1740, 97, 82, 97, 80, 176, 94, 96, 82, 98, 167, 180, 179, 102, 81, 97, 81, 98, 81, 97,
            81, 98, 81, 176, 178, 102, 82, 175, 179, 181, 178, 102, 80, 177, 93, 98, 80, 98, 167,
            101, 82, 97, 82, 256, 28703,
        ];
        let chunk = &SetTimesBoth::<22u32>::from_raw(&times_set)[0];
        let manchester_element_set = chunk.convert_to_manchester().unwrap();
        let frame = manchester_element_set.collect_frame().unwrap();
        assert_eq!(frame, Frame::StandardCrc(vec![0xA3]));
    }
}
