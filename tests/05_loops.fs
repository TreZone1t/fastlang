// Valid: while and loop
fn sum_loop(count: int(32)) -> int(32) {
    let int(32) i = 0;
    let int(32) total = 0;
    while (i < count) -> {
        set total -> total + 1;
        set i -> i + 1;
    }
    return total;
}
