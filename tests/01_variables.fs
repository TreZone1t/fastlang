// 01_variables.fs
// Test primitive types and variable declarations

 int(32) global_counter= 0;
 const float(64) pi = 3.14;

fn main() -> int(32) {
    //the feature i want to add:
    //int(32) a, b, c;
    int(32) a = 10 , b = 20 , c = 30;
    int(32) z , x, y = 10;
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
    /*
    Successfully generated C++ code to tests\build\output.cpp
Compiling to tests\build\app.exe...
tests\build\output.cpp: In function 'int main()':
tests\build\output.cpp:38:18: error: unable to deduce 'std::initializer_list<auto>*' from '<brace-enclosed initializer list>()'
   38 |     auto* ptr = {};
      |                  ^
tests\build\output.cpp:38:18: note:   couldn't deduce template parameter 'auto'
C++ compilation failed! Check tests\build\output.cpp for errors.
    
    */
    log(c);
    return 0;
}
