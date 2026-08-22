// 01_variables.fs
// Test primitive types and variable declarations

 int(32) global_counter= 0;
 const float(64) pi = 3.14;
fn make_name() -> name<int(32)> {
    int(32) a = 10;
    return &a;
}
fn main() -> int(32) {
    //the feature i want to add:
    //int(32) a, b, c;
    //int(32) a = 10 , b = 20 , c = 30;
    //int(32) z , x, y = 10;
    bool Is_active = true;
    char Initial = 'A';
    Is_active = false;
    Initial = 'B';
    int(32) c;
    int(32) counter = 0;
    counter = counter + 1;
    counter +=  1;
    c = counter;
    int(32) *ptr;
    ptr = &counter;
    *ptr = counter + 1;
    //name<int(32)> ptr2 = make_name(); // this is not working yet
    name<int(32)> ptr2;
    //ptr2 = &counter;  // this is not working yet for the same reason i think
    //log(ptr2);
    int a = 10; // you can use int as usual
    float b = 10.0; // you can use float as usual
    log(a);
    log(counter);
    log(c);
    return 0;
}
