use std::thread;
use std::time::Duration;

fn main() {
    let simulated_user_specified_value = 10;
    let simulated_random_number = 7;
    gererate_workout(simulated_user_specified_value, simulated_random_number);
    println!("====================");

    let x = vec![1, 2, 3];
    //let equal_to_x = move |z : Vec<i32>| z == x;
    //can't use x here
    //println!("Can't use x here: {:?}", x);
    println!("====================");
}

struct Cacher<T>
where
    // Fn(u32) -> u32: Trait & T: Fn(u32) -> u32: Trait bound
    T: Fn(u32) -> u32,
{
    calculation: T,
    value: Option<u32>,
}

impl<T> Cacher<T>
where
    T: Fn(u32) -> u32,
{
    fn new(calculation: T) -> Cacher<T> {
        Cacher {
            calculation,
            value: None,
        }
    }

    fn value(&mut self, arg: u32) -> u32 {
        match self.value {
            Some(v) => v,
            None => {
                //to use hashmap to store the value
                // let map = hashmap::HashMap::new();
                // map.insert(arg, (self.calculation)(arg));
                let v = (self.calculation)(arg);
                self.value = Some(v);
                v
            }
        }
    }
}

fn gererate_workout(intensity: u32, random_number: u32) {
    let mut expensive_closure = Cacher::new(|num| {
        println!("Calculating slowly...");
        thread::sleep(Duration::from_secs(2));
        num
    });

    if intensity < 25 {
        println!("Today, do {} pushups!", expensive_closure.value(intensity));
        
        println!("Next, do {} situps!", expensive_closure.value(intensity));
    } else {
        if random_number == 3 {
            println!("Take a break today! Remember to stay hydrated!");
        } else {
            println!("Today, run for {} minutes!", expensive_closure.value(intensity));
        }
    }
}


