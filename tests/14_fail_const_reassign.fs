// Invalid: Reassign const
fn main() -> int(32) {
    const int(32) x -> 5;
     x = 10; // Error: cannot reassign const
    return 0;
}
