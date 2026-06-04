fn main() {
    let mut  arr: Vec<i32> = vec![1, 2, 3];
    println!("{}", (arr.len() as i32).to_string());
    println!("{}", arr.is_empty().to_string());
    let mut  arr2: Vec<i32> = arr.clone();
    arr2.push(4);
    arr2.push(5);
    println!("{}", arr2[0 as usize].to_string());
    println!("{}", arr2[4 as usize].to_string());
    println!("{}", (arr2.len() as i32).to_string());
    println!("{}", (arr.len() as i32).to_string());
    println!("{}", arr2.pop().expect("empty").to_string());
}

