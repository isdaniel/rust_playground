//! # CargoLibrary
//!
//! `CargoLibrary` is a collection of utilities to make performing certain calculations more convenient.
//! cargo new CargoLibrary --lib

/// Adds two to the input number.
/// 
/// # Examples
/// ```
/// let result = CargoLibrary::add_two(2);
/// let answer = 4;
/// 
/// assert_eq!(result, answer);
/// ```
/// # Panics
/// 
/// # Errors
/// 
/// # Safety
/// 
/// # Abstraction
pub fn add_two(a: i32) -> i32 {
    a + 2
}


pub use self::Kinds::*;
pub use self::utils::*;

pub mod Kinds {
    pub enum PrimaryColors {
        Red,
        Blue,
        Yellow,
    }

    pub enum SecondaryColors {
        Orange,
        Green,
        Purple,
    }
}

pub mod utils {
    use crate::Kinds::*;

    /// Mixes two primary colors to produce a secondary color.
    /// a secondary color is produced by mixing two primary colors.
    pub fn mix(c1: PrimaryColors, c2: PrimaryColors) -> SecondaryColors {
        SecondaryColors::Orange
    }
}

