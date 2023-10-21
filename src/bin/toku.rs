use toku::rope::Rope;

fn main() {
    let rope = Rope::empty();
    println!("{}", rope.to_bstring());
}
