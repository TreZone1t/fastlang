// Invalid: Reassign const
fn main() -> int(32) {
    const int(32) x = 5;
    set x -> 10; // Error: cannot reassign const
    return 0;
}
