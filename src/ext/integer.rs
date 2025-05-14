use std::ops::DivAssign;

pub trait IntegerExt: Sized {
    fn div_assign_10(&mut self);
    fn is_zero(&self) -> bool;

    /// Count the number of digits for an integer.
    ///
    /// If the number is negative, it should not count the negative symbol.
    fn digits(self) -> u32 {
        self.checked_digits().unwrap()
    }

    fn checked_digits(mut self) -> Option<u32> {
        if self.is_zero() {
            Some(1)
        } else {
            let mut digits: u32 = 1;

            loop {
                self.div_assign_10();
                if self.is_zero() {
                    break Some(digits);
                }

                digits += 1;
            }
        }
    }
}

macro_rules! impl_integer_ext_for {
    ($ty:ty) => {
        impl IntegerExt for $ty {
            fn div_assign_10(&mut self) {
                self.div_assign(10);
            }

            fn is_zero(&self) -> bool {
                0.eq(self)
            }
        }
    }
}
impl_integer_ext_for!(i8);
impl_integer_ext_for!(i16);
impl_integer_ext_for!(i32);
impl_integer_ext_for!(i64);
impl_integer_ext_for!(i128);
impl_integer_ext_for!(isize);
impl_integer_ext_for!(u8);
impl_integer_ext_for!(u16);
impl_integer_ext_for!(u32);
impl_integer_ext_for!(u64);
impl_integer_ext_for!(u128);
impl_integer_ext_for!(usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_digits() {
        fn test_num(i: i32, digits: u32) {
            assert_eq!(i.digits(), digits);
            assert_eq!((-i).digits(), digits);
        }

        test_num(0, 1);
        test_num(1, 1);
        test_num(10, 2);
        test_num(100, 3);
    }
}
