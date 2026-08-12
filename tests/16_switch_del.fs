

fn main() -> int(32) {
    // Test switch
    let int(32) value = 2;
    let int(32) x = 0;
    let int(32) y = 0;
    let int(32) z = 0;
    
    switch (value) -> {
        case 1 => {
            set x -> 1;
        }
        case 2 => {
             set y -> 2;
        }
        _ => {
            set z -> 3;
        }
    }
    log(x);
    log(y);
    log(z);
    return 0;
}
