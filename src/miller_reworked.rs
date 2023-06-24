#[cfg(feature = "std")]
use std::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use bitvec::prelude::{BitVec, Lsb0};

use crate::error::MillerError;
use crate::frame::{CompleteCollector, Frame};
//use crate::time_record_both_ways::{EntryTimesBoth, SetTimesBoth};

impl Frame {
    pub fn process_buffer_miller_skip_tails<P, const TICK_LEN: u16>(buffer: &[u16], frame_filter: P) -> Vec<Self>
        where P: Fn(&Self) -> bool
    {
        let iter = buffer.split(|interval| *interval > 19 * TICK_LEN);
        let iter_len = buffer.split(|interval| *interval > 19 * TICK_LEN).count();
        if iter_len > 2 {
            let mut frames_set = Vec::new();
            for times_set in iter.skip(1).take(iter_len-2) {
                let mut miller_element_set = MillerElementSet::new();
                let mut flag_not_miller = false;
                for time_interval in times_set.into_iter() {
                    if let Err(_) = miller_element_set.add_time_down_interval::<TICK_LEN>(*time_interval) {
                        flag_not_miller = true;
                        break;
                    };
                }
                if flag_not_miller {break;}
                match miller_element_set.element_set.last() {
                    None => {},
                    Some(MillerElement::X) => {
                        miller_element_set.element_set.push(MillerElement::Y);
                        miller_element_set.element_set.push(MillerElement::Y);
                    },
                    Some(MillerElement::Y) => unreachable!(),
                    Some(MillerElement::Z) => miller_element_set.element_set.push(MillerElement::Y),
                }
                if let Ok(frame) = miller_element_set.collect_frame() {
                    if frame_filter(&frame) {
                        frames_set.push(frame)
                    }
                }
            }
            frames_set
        }
        else {Vec::new()}
    }
}

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
    Complete(Frame),
}

impl MillerCollector {
    pub fn add_element(&mut self, element: MillerElement) -> Result<(), MillerError> {
        match self {
            MillerCollector::Empty => {
                if let MillerElement::Z = element {
                    *self = MillerCollector::InProgress(BitVec::<u8, Lsb0>::new())
                } else {
                    return Err(MillerError::WrongMillerSequence);
                }
            }
            MillerCollector::InProgress(set) => {
                let last_bit = set.last().map(|bitref| *bitref);
                match element {
                    MillerElement::X => {
                        set.push(true);
                    }
                    MillerElement::Y => match last_bit {
                        None => return Err(MillerError::WrongMillerSequence),
                        Some(false) => {
                            let collector = CompleteCollector{data: &set[..set.len() - 1]};
                            let frame = collector.to_frame().map_err(MillerError::Frame)?;
                            *self = MillerCollector::Complete(frame)
                        }
                        Some(true) => {
                            set.push(false);
                        }
                    },
                    MillerElement::Z => {
                        if let Some(true) = last_bit {
                            return Err(MillerError::WrongMillerSequence);
                        } else {
                            set.push(false);
                        }
                    }
                }
            }
            MillerCollector::Complete(_) => return Err(MillerError::WrongMillerSequence),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MillerTimesDown<'a, const TICK_LEN: u16> {
    time_down_set: &'a [u16],
}

impl<'a, const TICK_LEN: u16> MillerTimesDown<'a, TICK_LEN> {

    pub fn convert(self) -> Result<Frame, MillerError> {
        // each bit length is 8 ticks; expected error is 1 tick;
        // time intervals in off mode are identical throughout the code;
        // no signal corresponds to 2 or more completely "on" bits, i.e. 16 ticks.
        // since only times down are recorded, the selected slices are separated by 16+4 = 20 ticks.
        // minimal 19 ticks are taken as the error is expected to be 1 tick;
        // XYY (20 ticks) or ZYY (24 ticks)
        let mut miller_element_set = MillerElementSet::new();
        for time_interval in self.time_down_set.into_iter() {
            miller_element_set.add_time_down_interval::<TICK_LEN>(*time_interval)?;
        }
        match miller_element_set.element_set.last() {
            None => {},
            Some(MillerElement::X) => {
                miller_element_set.element_set.push(MillerElement::Y);
                miller_element_set.element_set.push(MillerElement::Y);
            },
            Some(MillerElement::Y) => unreachable!(),
            Some(MillerElement::Z) => miller_element_set.element_set.push(MillerElement::Y),
        }
        miller_element_set.collect_frame()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MillerElementSet {
    pub element_set: Vec<MillerElement>,
}

impl MillerElementSet {
    pub fn new() -> Self {
        Self {
            element_set: Vec::new(),
        }
    }

    fn process_previous_x<const TICK_LEN: u16>(
        &mut self,
        interval: u16,
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
            return Err(MillerError::UnexpectedInterval(interval));
        }
        Ok(())
    }

    fn process_previous_z<const TICK_LEN: u16>(
        &mut self,
        interval: u16,
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
            return Err(MillerError::UnexpectedInterval(interval));
        }
        Ok(())
    }

    fn add_time_down_interval<const TICK_LEN: u16>(
        &mut self,
        interval: u16,
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
/*
    pub(crate) fn add_time_both_interval<const TICK_LEN: u16>(
        &mut self,
        time_both: EntryTimesBoth,
    ) -> Result<(), MillerError> {
        if (time_both.first_len < 3 * TICK_LEN) | (time_both.first_len > 5 * TICK_LEN) {
            return Err(MillerError::UnexpectedMillerOffInterval(time_both.first_len));
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
    pub fn from_times_both<const TICK_LEN: u16>(
        times_both: SetTimesBoth<TICK_LEN>,
    ) -> Result<Self, MillerError> {
        times_both.convert_to_miller()
    }
*/
    pub fn collect_frame(self) -> Result<Frame, MillerError> {
        let mut collector = MillerCollector::Empty;
        for element in self.element_set.into_iter() {
            collector.add_element(element)?;
        }
        if let MillerCollector::Complete(complete_collector) = collector {
            Ok(complete_collector)
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
