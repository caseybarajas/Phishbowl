use world::Axis;

/// Scale a base weight by a personality axis, where 50 is neutral (factor 1.0).
/// A 0..=100 axis maps a weight to roughly 0..=2x. Result is bounded well within i16.
pub fn by_axis(weight: i16, axis: Axis) -> i16 {
    let scaled = i32::from(weight) * i32::from(axis.get()) / 50;
    i16::try_from(scaled.clamp(-1000, 1000)).expect("bounded weight")
}

/// Fraction of a base weight, 0 at axis 0 and the full weight at axis 100.
pub fn by_axis_fraction(weight: i16, axis: Axis) -> i16 {
    let scaled = i32::from(weight) * i32::from(axis.get()) / 100;
    i16::try_from(scaled.clamp(-1000, 1000)).expect("bounded weight")
}
