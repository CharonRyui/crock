use crate::clock::timer::Timer;

pub mod error;
pub mod timer;

#[derive(Debug)]
pub struct Clock {
    timer: Timer,
}
