macro_rules! clamped {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(u8);

        impl $name {
            pub fn new(value: u8) -> Self {
                Self(value.min(100))
            }

            pub fn get(self) -> u8 {
                self.0
            }

            #[must_use]
            pub fn apply(self, delta: i16) -> Self {
                let next = (i16::from(self.0) + delta).clamp(0, 100);
                Self(u8::try_from(next).expect("clamped to 0..=100"))
            }
        }
    };
}

clamped!(Suspicion);
clamped!(Trust);
clamped!(Sensitivity);
clamped!(Axis);
