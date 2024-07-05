use Generic::{Tweet,Summary};

fn largetst<T>(list: &[T]) -> T
    where T: PartialOrd + Copy {
    let mut largest = list[0];
    for &item in list.iter() {
        if item > largest {
            largest = item;
        }
    }
    largest
}

struct Point<T,U> {
    x: T,
    y: U,
}

impl<T,U> Point<T,U> {
    fn x(&self) -> &T {
        &self.x
    }
}

impl<T,U> Point<T,U> {
    fn mixup<V,W>(self, other: Point<V,W>) -> Point<T,W> {
        Point {
            x: self.x,
            y: other.y,
        }
    }
}


fn main() {
    let number_list = vec![34, 50, 25, 100, 65];
    let result = largetst::<i32>(&number_list);
    println!("The largest number is {}", result);

    let char_list = vec!['y', 'm', 'a', 'q'];
    let result = largetst::<char>(&char_list);
    println!("The largest number is {}", result);

    println!("==================");

    let p = Point { x: 5, y: 10 };
    println!("p.x = {}", p.x());

    println!("==================");
    let p1 = Point { x: 5, y: 10.4 };
    let p2 = Point { x: "Hello", y: 'c' };
    let p3 = p1.mixup(p2);
    println!("p3.x = {}, p3.y = {}", p3.x, p3.y);
    println!("==================");

    let tweet = Tweet {
        username: String::from("horse_ebooks"),
        content: String::from("of course, as you probably already know, people"),
        reply: false,
        retweet: false,
    };
    print!("1 new tweet: {}\r\n", tweet.summarize());
    print!("{}", tweet.default_summarize());
}
