#[derive(PartialEq, Debug)]
pub struct Shoe {
    size: u32,
    style: String,
}

fn shoes_in_my_size(shoes: Vec<Shoe>, shoe_size: u32) -> Vec<Shoe> {
    shoes.into_iter().filter(|s| s.size == shoe_size).collect()
}
use crate::interator_lab::Counter;

#[cfg(test)]
mod tests{
    use crate::Shoe;
    
    #[test]
    fn iterator_demonstration(){
        let v1 = vec![1, 2, 3];
        let mut v1_iter = v1.iter();
        
        assert_eq!(v1_iter.next(), Some(&1));
        assert_eq!(v1_iter.next(), Some(&2));
        assert_eq!(v1_iter.next(), Some(&3));
    }
    #[test]
    fn iterator_sum(){
        let v1 = vec![1, 2, 3];
        let v1_iter = v1.iter();
        
        assert_eq!(v1_iter.sum::<i32>(), 6);
    }

    #[test]
    fn iterator_map(){
        let v1 = vec![1, 2, 3];
        let v2: Vec<_> = v1.iter().map(|x| x + 1).collect();
        assert_eq!(v2, vec![2, 3, 4]);
    }

    #[test]
    fn iterator_filter(){
        let shoes = vec![
            Shoe{size: 10, style: String::from("sneaker")},
            Shoe{size: 13, style: String::from("sandal")},
            Shoe{size: 10, style: String::from("boot")},
        ];

        let in_my_size = super::shoes_in_my_size(shoes, 10);
        assert_eq!(
            in_my_size,
            vec![
                Shoe{size: 10, style: String::from("sneaker")},
                Shoe{size: 10, style: String::from("boot")},
            ]
        );
    }
    
    #[test]
    fn calling_next_directly(){
        let mut counter = Counter::new();
        
        assert_eq!(counter.next(), Some(1));
        assert_eq!(counter.next(), Some(2));
        assert_eq!(counter.next(), Some(3));
        assert_eq!(counter.next(), Some(4));
        assert_eq!(counter.next(), Some(5));
        assert_eq!(counter.next(), None);
    }
    

}