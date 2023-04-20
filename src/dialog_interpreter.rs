//! Interpreter of a dialog between PCD and PICC device.
//!
//! It is assumed here that the PCD sends Miller-encoded data and PICC responds
//! with Manchester-encoded data.
//!
//! Some PCD frames require no response.
//!
//! All PICC frames are received as a response to PCD requests.

use crate::error::FrameError;
use crate::frame::{Frame, FrameAttributed};

/// ALL_REQ, expected as short Miller frame
pub const ALL_REQ: FrameAttributed = FrameAttributed::Miller(Frame::Short(0x52));

/// SENS_REQ, expected as short Miller frame
pub const SENS_REQ: FrameAttributed = FrameAttributed::Miller(Frame::Short(0x26));

pub const SPL_REQ: [u8;2] = [0x50, 0x00];

pub fn combine_sdd_clean_cut(miller_frame_date: &[u8], manchester_frame_date: &[u8]) -> Result<[u8; 7], FrameError> {
    Ok([0;7])
}
