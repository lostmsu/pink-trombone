use std::ops::{Add, Mul, Sub};

pub fn interpolate<T>(i0: T, i1: T, v: T) -> T
where
    T: Sub<T, Output = T> + Mul<T, Output = T> + Add<T, Output = T> + Copy,
{
    i0 + v * (i1 - i0)
}

pub fn move_towards(current: f64, target: f64, amount_up: f64, amount_down: f64) -> f64 {
    if current < target {
        target.min(current + amount_up)
    } else {
        target.max(current - amount_down)
    }
}

pub fn sqr<T>(x: T) -> T
where
    T: Mul<T, Output = T> + Copy,
{
    x * x
}
