fn main() {
    let names = vec!["Circle", "Square", "C", "LLVM", "Native"];
    for name in names {
        let is_cap = name.chars().next().map_or(false, |c| c.is_uppercase());
        println!("{}: {}", name, is_cap);
    }
}