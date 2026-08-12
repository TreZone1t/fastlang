// 01_variables.fs
// Test primitive types and variable declarations
use std {array, str};
let int(32) global_counter= 0;
const float(64) pi -> 3.14;

fn main() -> int(32) {
    let bool is_active = true;
    let char initial = 'A';
    str message -> "Hello";
   array<int(32)> numbers -> [1, 2, 3];
    
    return 0;
}
