

fn main() -> int(32) {
    // Test match
    int(32) value = 2;
    int(32) x = 0;
    int(32) y = 0;
    int(32) z = 0;
    
    match (value) -> {
        1 => {
            x = 1;
        }
        2 => {
            y = 2;
        }
        _ => {
            z = 3;
        }
    }
    log(x);
    log(y);
    log(z);
    return 0;
}
