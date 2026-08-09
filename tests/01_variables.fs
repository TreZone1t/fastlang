// 01_variables.fs
// Test primitive types and variable declarations

let int(32) global_counter = 0;
const float(64) pi = 3.14;

fn main() -> int(32) {
    let bool is_active = true;
    let char initial = 'A';
    let str(255) message = "Hello";
    let array(int(32) ,3) numbers = [1, 2, 3];
    
    return 0;
}
