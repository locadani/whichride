//! Pricing rules for comparing the cost of one carsharing trip across providers.
//!
//! The crate is pure: no I/O, no clock, no floating point. The Android app, the website and the
//! server all call the same functions, so their numbers can never disagree.
