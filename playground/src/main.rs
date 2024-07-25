use std::rc::Rc;

struct Truck{
    capacity: u32,
}

fn main() {
    let (truck_a,truck_b,truck_c) = (
        Rc::new(Truck{capacity : 1}),
        Rc::new(Truck{capacity : 2}),
        Rc::new(Truck{capacity : 3}),
    );
    let facility_one = vec![Rc::clone(&truck_a),Rc::clone(&truck_b)];
    let facility_one = vec![truck_b,truck_c];

    println!("Truck A capacity: {}", truck_a.capacity);
}
