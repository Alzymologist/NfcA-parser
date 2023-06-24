#![no_std]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;

#[cfg(not(feature = "std"))]
extern crate core;
#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod error;
pub mod frame;
//pub mod manchester;
pub mod miller;
pub mod miller_reworked;
//pub mod time_record_both_ways;
