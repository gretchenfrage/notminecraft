
use std::time::Duration;


pub fn secs_rem(dividend: Duration, divisor_secs: f32) -> f32 {
    const INVERSE_NANO: u128 = 1_000_000_000;
    let divisor = Duration::from_secs_f32(divisor_secs);
    let remainder_ns = dividend.as_nanos() % divisor.as_nanos();
    Duration::new(
        (remainder_ns / INVERSE_NANO) as u64,
        (remainder_ns % INVERSE_NANO) as u32,
    ).as_secs_f32()
}
