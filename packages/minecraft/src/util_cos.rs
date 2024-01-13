//! Cosine wave utility.

use std::f32;


/// Utility for moving some value along a cosine wave. Automatically wraps
/// input around period to prevent precision-loss as values get high.
#[derive(Debug, Clone)]
pub struct Cosine {
    input: f32,
    period: f32,
}

impl Cosine {
    /// Create a new cosine wave at starting value 0 with the given period.
    pub fn new(period: f32) -> Self {
        Cosine {
            input: 0.0,
            period,
        }
    }

    /// Add the given value to the input.
    pub fn add_to_input(&mut self, n: f32) {
        self.input += n;
        self.input %= self.period;
    }

    /// Get the output of the cosine wave at the current input value.
    pub fn get(&self) -> f32 {
        (self.input * f32::consts::PI / self.period).cos()
    }
}
