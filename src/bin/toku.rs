use toku::rope::Rope;

fn main() {
    let rope = Rope::from("Hello, world!");
    println!("{}", rope.to_string());
}
