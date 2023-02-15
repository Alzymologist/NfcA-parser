use bitvec::prelude::{BitVec, Lsb0};

use crate::error::MillerError;
use crate::frame::{CompleteCollector, Frame};
use crate::time_record_both_ways::{EntryTimesBoth, SetTimesBoth};

#[derive(Debug, Eq, PartialEq)]
pub enum MillerElement {
    X,
    Y,
    Z,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MillerCollector {
    Empty,
    InProgress(BitVec<u8, Lsb0>),
    Complete(CompleteCollector),
}

impl MillerCollector {
    pub fn add_element(&mut self, element: &MillerElement) -> Result<(), MillerError> {
        match self {
            MillerCollector::Empty => {
                if let MillerElement::Z = element {
                    *self = MillerCollector::InProgress(BitVec::<u8, Lsb0>::new())
                } else {
                    return Err(MillerError::WrongMillerSequence);
                }
            }
            MillerCollector::InProgress(set) => {
                let last_bit = {
                    if set.is_empty() {
                        None
                    } else {
                        Some(set[set.len() - 1])
                    }
                };
                match element {
                    MillerElement::X => {
                        set.push(true);
                        *self = MillerCollector::InProgress(set.to_bitvec())
                    }
                    MillerElement::Y => match last_bit {
                        None => return Err(MillerError::WrongMillerSequence),
                        Some(false) => {
                            *self = MillerCollector::Complete(CompleteCollector {
                                data: set[..set.len() - 1].to_bitvec(),
                            })
                        }
                        Some(true) => {
                            set.push(false);
                            *self = MillerCollector::InProgress(set.to_bitvec())
                        }
                    },
                    MillerElement::Z => {
                        if let Some(true) = last_bit {
                            return Err(MillerError::WrongMillerSequence);
                        } else {
                            set.push(false);
                            *self = MillerCollector::InProgress(set.to_bitvec())
                        }
                    }
                }
            }
            MillerCollector::Complete(_) => return Err(MillerError::WrongMillerSequence),
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MillerTimesDown<const TICK_LEN: u32> {
    time_down_set: Vec<u32>,
}

impl<const TICK_LEN: u32> MillerTimesDown<TICK_LEN> {
    pub fn from_raw(time_down_input: &[u32]) -> Vec<Self> {
        time_down_input
            .split(|interval| *interval > 19 * TICK_LEN)
            .map(|slice| MillerTimesDown::<TICK_LEN> {
                time_down_set: slice.to_vec(),
            })
            .collect()
    }

    pub fn convert(&self) -> Result<MillerElementSet, MillerError> {
        // each bit length is 8 ticks; expected error is 1 tick;
        // time intervals in off mode are identical throughout the code;
        // no signal corresponds to 2 or more completely "on" bits, i.e. 16 ticks.
        // since only times down are recorded, the selected slices are separated by 16+4 = 20 ticks.
        // minimal 19 ticks are taken as the error is expected to be 1 tick;
        // XYY (20 ticks) or ZYY (24 ticks)
        let mut miller_element_set = MillerElementSet::new();
        for time_interval in self.time_down_set.iter() {
            miller_element_set.add_time_down_interval::<TICK_LEN>(*time_interval)?;
        }
        Ok(miller_element_set)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MillerElementSet {
    element_set: Vec<MillerElement>,
}

impl MillerElementSet {
    pub fn new() -> Self {
        Self {
            element_set: Vec::new(),
        }
    }

    fn process_previous_x<const TICK_LEN: u32>(
        &mut self,
        interval: u32,
    ) -> Result<(), MillerError> {
        if (interval >= 7 * TICK_LEN) & (interval <= 9 * TICK_LEN) {
            self.element_set.push(MillerElement::X)
        } else if (interval >= 11 * TICK_LEN) & (interval <= 13 * TICK_LEN) {
            self.element_set.push(MillerElement::Y);
            self.element_set.push(MillerElement::Z);
        } else if (interval >= 15 * TICK_LEN) & (interval <= 17 * TICK_LEN) {
            self.element_set.push(MillerElement::Y);
            self.element_set.push(MillerElement::X);
        } else {
            return Err(MillerError::UnexpectedInterval);
        }
        Ok(())
    }

    fn process_previous_z<const TICK_LEN: u32>(
        &mut self,
        interval: u32,
    ) -> Result<(), MillerError> {
        if (interval >= 7 * TICK_LEN) & (interval <= 9 * TICK_LEN) {
            self.element_set.push(MillerElement::Z)
        } else if (interval >= 11 * TICK_LEN) & (interval <= 13 * TICK_LEN) {
            self.element_set.push(MillerElement::X)
        } else if (interval >= 15 * TICK_LEN) & (interval <= 17 * TICK_LEN) {
            // sequence ZYZ is invalid and will be
            // sieved out during further processing
            self.element_set.push(MillerElement::Y);
            self.element_set.push(MillerElement::Z);
        } else {
            return Err(MillerError::UnexpectedInterval);
        }
        Ok(())
    }

    fn add_time_down_interval<const TICK_LEN: u32>(
        &mut self,
        interval: u32,
    ) -> Result<(), MillerError> {
        // intervals above 20 ticks are expected to be eliminated at this point
        match self.element_set.last() {
            None => {
                self.element_set.push(MillerElement::Z);
                self.process_previous_z::<TICK_LEN>(interval)
            }
            Some(MillerElement::X) => self.process_previous_x::<TICK_LEN>(interval),
            Some(MillerElement::Y) => unreachable!(),
            Some(MillerElement::Z) => self.process_previous_z::<TICK_LEN>(interval),
        }
    }

    pub(crate) fn add_time_both_interval<const TICK_LEN: u32>(
        &mut self,
        time_both: EntryTimesBoth,
    ) -> Result<(), MillerError> {
        if (time_both.first_len < 3 * TICK_LEN) | (time_both.first_len > 5 * TICK_LEN) {
            return Err(MillerError::UnexpectedMillerOffInterval);
        }
        // intervals above 16 ticks are expected to be eliminated at this point
        match self.element_set.last() {
            None => {
                self.element_set.push(MillerElement::Z);
                match time_both.second_len {
                    None => {
                        self.element_set.push(MillerElement::Y);
                        Ok(())
                    }
                    Some(second_len) => {
                        let interval = second_len + time_both.first_len;
                        self.process_previous_z::<TICK_LEN>(interval)
                    }
                }
            }
            Some(MillerElement::X) => {
                match time_both.second_len {
                    None => {
                        // XY is invalid final sequence, is filtered out on further processing
                        self.element_set.push(MillerElement::Y);
                        Ok(())
                    }
                    Some(second_len) => {
                        let interval = second_len + time_both.first_len;
                        self.process_previous_x::<TICK_LEN>(interval)
                    }
                }
            }
            Some(MillerElement::Y) => unreachable!(),
            Some(MillerElement::Z) => match time_both.second_len {
                None => {
                    self.element_set.push(MillerElement::Y);
                    Ok(())
                }
                Some(second_len) => {
                    let interval = second_len + time_both.first_len;
                    self.process_previous_z::<TICK_LEN>(interval)
                }
            },
        }
    }

    pub fn from_times_down<const TICK_LEN: u32>(
        times_down: MillerTimesDown<TICK_LEN>,
    ) -> Result<Self, MillerError> {
        times_down.convert()
    }

    pub fn from_times_both<const TICK_LEN: u32>(
        times_both: SetTimesBoth<TICK_LEN>,
    ) -> Result<Self, MillerError> {
        times_both.convert_to_miller()
    }

    pub fn collect_frame(&self) -> Result<Frame, MillerError> {
        let mut collector = MillerCollector::Empty;
        for element in self.element_set.iter() {
            collector.add_element(element)?;
        }
        if let MillerCollector::Complete(complete_collector) = collector {
            complete_collector.to_frame().map_err(MillerError::Frame)
        } else {
            Err(MillerError::IncompleteFrame)
        }
    }
}

impl Default for MillerElementSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::bitvec;

    #[test]
    fn miller_collector_01() {
        let mut collector = MillerCollector::Empty;
        assert!(collector.add_element(&MillerElement::X).is_err());
        assert!(collector.add_element(&MillerElement::Y).is_err());
        collector.add_element(&MillerElement::Z).unwrap();
        assert_eq!(
            collector,
            MillerCollector::InProgress(BitVec::<u8, Lsb0>::new())
        );
    }

    #[test]
    fn miller_collector_02() {
        let mut collector = MillerCollector::InProgress(BitVec::<u8, Lsb0>::new());
        collector.add_element(&MillerElement::X).unwrap();
        assert_eq!(collector, MillerCollector::InProgress(bitvec![u8, Lsb0; 1]));
    }

    #[test]
    fn miller_collector_03() {
        let mut collector = MillerCollector::InProgress(BitVec::<u8, Lsb0>::new());
        assert!(collector.add_element(&MillerElement::Y).is_err());
    }

    #[test]
    fn miller_collector_04() {
        let mut collector = MillerCollector::InProgress(BitVec::<u8, Lsb0>::new());
        collector.add_element(&MillerElement::Z).unwrap();
        assert_eq!(collector, MillerCollector::InProgress(bitvec![u8, Lsb0; 0]));
    }

    #[test]
    fn miller_collector_05() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&MillerElement::X).unwrap();
        assert_eq!(
            collector,
            MillerCollector::InProgress(bitvec![u8, Lsb0; 0, 1])
        );
    }

    #[test]
    fn miller_collector_06() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&MillerElement::Y).unwrap();
        assert_eq!(
            collector,
            MillerCollector::Complete(CompleteCollector {
                data: BitVec::<u8, Lsb0>::new()
            })
        );
    }

    #[test]
    fn miller_collector_07() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 0]);
        collector.add_element(&MillerElement::Z).unwrap();
        assert_eq!(
            collector,
            MillerCollector::InProgress(bitvec![u8, Lsb0; 0, 0])
        );
    }

    #[test]
    fn miller_collector_08() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 1]);
        collector.add_element(&MillerElement::X).unwrap();
        assert_eq!(
            collector,
            MillerCollector::InProgress(bitvec![u8, Lsb0; 1, 1])
        );
    }

    #[test]
    fn miller_collector_09() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 1]);
        collector.add_element(&MillerElement::Y).unwrap();
        assert_eq!(
            collector,
            MillerCollector::InProgress(bitvec![u8, Lsb0; 1, 0])
        );
    }

    #[test]
    fn miller_collector_10() {
        let mut collector = MillerCollector::InProgress(bitvec![u8, Lsb0; 1]);
        assert!(collector.add_element(&MillerElement::Z).is_err());
    }

    #[test]
    fn miller_sequence_1() {
        let sequence = [
            MillerElement::Z,
            MillerElement::X,
            MillerElement::Y,
            MillerElement::Z,
            MillerElement::X,
            MillerElement::Y,
            MillerElement::Y,
        ];
        let mut collector = MillerCollector::Empty;
        for element in sequence {
            collector.add_element(&element).unwrap();
        }
        assert_eq!(
            collector,
            MillerCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 1, 0, 0, 1]
            })
        );
    }

    #[test]
    fn miller_sequence_2() {
        let sequence = [
            MillerElement::Z,
            MillerElement::X,
            MillerElement::Y,
            MillerElement::Z,
            MillerElement::Y,
        ];
        let mut collector = MillerCollector::Empty;
        for element in sequence {
            collector.add_element(&element).unwrap();
        }
        assert_eq!(
            collector,
            MillerCollector::Complete(CompleteCollector {
                data: bitvec![u8, Lsb0; 1, 0]
            })
        );
    }

    #[test]
    fn miller_time_both_1() {
        let times_set = [
            25001, 82, 101, 75, 191, 80, 102, 75, 191, 79, 189, 80, 189, 80, 1734,
        ];
        let chunk = &SetTimesBoth::<22u32>::from_raw(&times_set)[0];
        let miller_element_set = chunk.convert_to_miller().unwrap();
        let frame = miller_element_set.collect_frame().unwrap();
        assert_eq!(frame, Frame::Short(0x26));
    }

    #[test]
    fn miller_time_both_2() {
        let times_set = [
            58364, 72, 110, 68, 198, 71, 198, 71, 198, 71, 109, 68, 289, 71, 110, 68, 111, 68, 110,
            68, 111, 68, 198, 71, 198, 71, 109, 69, 198, 71, 198, 71, 110, 68, 110, 68, 199, 71,
            110, 68, 199, 70, 110, 68, 199, 71, 1737,
        ];
        let chunk = &SetTimesBoth::<22u32>::from_raw(&times_set)[0];
        let miller_element_set = chunk.convert_to_miller().unwrap();
        let frame = miller_element_set.collect_frame().unwrap();
        assert_eq!(frame, Frame::Standard(vec![0xB2]));
    }
}
