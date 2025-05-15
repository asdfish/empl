use std::num::ParseIntError;

pub trait FromStrRadix: Sized {
    fn from_str_radix(_: &str, _: u32) -> Result<Self, ParseIntError>;
}

macro_rules! impl_from_str_radix_for {
    ($ty:ty) => {
        impl FromStrRadix for $ty {
            fn from_str_radix(int: &str, radix: u32) -> Result<Self, ParseIntError> {
                <$ty>::from_str_radix(int, radix)
            }
        }
    };
}
impl_from_str_radix_for!(i8);
impl_from_str_radix_for!(i16);
impl_from_str_radix_for!(i32);
impl_from_str_radix_for!(i64);
impl_from_str_radix_for!(i128);
impl_from_str_radix_for!(isize);
impl_from_str_radix_for!(u8);
impl_from_str_radix_for!(u16);
impl_from_str_radix_for!(u32);
impl_from_str_radix_for!(u64);
impl_from_str_radix_for!(u128);
impl_from_str_radix_for!(usize);
