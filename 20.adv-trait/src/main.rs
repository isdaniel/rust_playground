use std::ops::Add;
use std::fmt;

struct MilliMeters(u32);

struct Meters(u32);

impl Add<Meters> for MilliMeters {
    type Output = MilliMeters;

    fn add(self, rhs: Meters) -> MilliMeters  {
        MilliMeters(self.0 + (rhs.0 * 1000))
    }
}


impl fmt::Display for MilliMeters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn main() {
    let mm100 = MilliMeters(100);
    let m10 = Meters(10);
    println!("{} mm" , mm100.add(m10));
}
