fn serve_order(){}
pub use crate::front_of_house::hosting;

mod back_of_house{
    pub struct Breakfast{
        pub toast: String,
        seasonal_fruit: String,
    }

    impl Breakfast{
        pub fn summer(toast: &str) -> Breakfast{
            Breakfast{
                toast: String::from(toast),
                seasonal_fruit: String::from("peaches"),
            }
        }
    }

    pub enum Appetizer{
        Soup,
        Salad,
    }
}

pub fn eat_at_reastaurant(){
    let mut meal = back_of_house::Breakfast::summer("Rye");
    meal.toast = String::from("Wheat");
    //meal.seasonal_fruit = String::from("blueberries");

    println!("I'd like {} toast please",meal.toast);

    let order1 = back_of_house::Appetizer::Soup;
    let order2 = back_of_house::Appetizer::Salad;
}

mod front_of_house{  
    pub mod hosting{
        fn add_to_waitlist(){}
        fn seat_at_table(){}
        fn fix_incorrect_order(){
            crate::serve_order();
            //crate::serve_order();
        }
    }

    mod serving{
        fn take_order(){}
        fn take_payment(){}
    }
}