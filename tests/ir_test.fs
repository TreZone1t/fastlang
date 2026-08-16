fn my_add(a: int(32), b: int(32)) -> int(32) {
    int(32) result = a + b;
    return result;
}

fn main() -> void {
    int(32) x = 10;
    int(32) y = 20;
    int(32) z = my_add(x, y);
}
