#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Date {
    inner: i64,
}

impl Date {
    fn new(nanosecs: i64) -> Self {
        let seconds = nanosecs.div_euclid(1_000_000_000);
        let minutes = seconds.div_euclid(60);
        let hours = minutes.div_euclid(60);
        let days = hours.div_euclid(24);
        Date { inner: days as i64 }
    }
    pub fn next(&self) -> Self {
        Date {
            inner: self.inner + 1,
        }
    }
}

macro_rules! impl_date_from {
    ($op:ident) => {
        impl From<$op> for Date {
            fn from(value: $op) -> Self {
                Self::new(value as i64)
            }
        }
    };
}

impl_date_from!(i64);
impl_date_from!(u64);
impl_date_from!(i32);

#[cfg(test)]
mod tests {
    use super::*;
    const SECONDS_IN_DAY: i64 = 86_400;

    const NANOSECONDS_IN_DAY: i64 = SECONDS_IN_DAY * 1_000_000_000;
    #[test]
    fn test_date_eq() {
        let date1 = Date::from(0);
        let date2 = Date::from(1);
        assert_eq!(date1, date2);
        let date3 = Date::from(NANOSECONDS_IN_DAY);
        assert!(!date1.eq(&date3));
        let date4 = Date::from(NANOSECONDS_IN_DAY - 1);
        assert_eq!(date1, date4);
    }
}
