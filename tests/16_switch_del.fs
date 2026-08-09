

fn main() -> void {
    // Test del statement
    let int(32) temp = 42;
    del temp;

    // Test switch
    let int(32) value = 2;
    
    switch (value) -> {
        case 1 => {
            let int(32) x = 1;
        }
        case 2 => {
            let int(32) y = 2;
        }
        _ => {
            let int(32) z = 3;
        }
    }
}
