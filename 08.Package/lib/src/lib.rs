pub fn add_lib(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add_lib(2, 2);
        assert_eq!(result, 4);
    }
}
