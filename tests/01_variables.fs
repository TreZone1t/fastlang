// 01_variables.fs
// Test primitive types and variable declarations

 int(32) global_counter= 0;
 const float(64) pi -> 3.14;

fn main() -> int(32) {
    bool Is_active = true;
    char Initial = 'A';
    Is_active = false;
    Initial = 'B';
    int(32) counter = 0;
    counter = counter + 1;
    counter +=  1;
    str message -> "Hello";
    set message -> "World";
    return 0;
}
