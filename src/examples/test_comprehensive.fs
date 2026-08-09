// ==========================================
// Fast Lang - Comprehensive Semantic Test
// ==========================================

let int(32) global_x = 100;
let string global_name = "fast_lang";

// 1. Classes and OOP Validation
class Animal -> {
    public -> {
        let string name = "unknown";
    }
    _() -> {
        set this.name -> "animal";
    }
}

class Dog extends Animal -> {
    public -> {
        let int(32) age = 0;
    }
    _() -> {
        set super.name -> "dog";
        set this.age -> 1;
    }
}

struct Point -> {
    public -> {
        let int(32) x = 0;
        let int(32) y = 0;
    }
    _() -> {
        set this.x -> 10;
        set this.y -> 20;
    }
}

// 2. Objects and Magic Types
let object dog_instance = new Dog();
let object dog_copy = copy dog_instance;
let object dog_mod = modify dog_instance;

let list(10) my_list = [1, 2, 3];
let string my_text = "hello world";

// Allowed Magic Types usages
let length list_len = my_list;
let size text_size = my_text;
let init point_init = Point;

// 3. Scopes, Parameters, and Statements
scope process_data -> {
    type -> block;
    param -> {
        let int(32) p1 = 0;
        let string p2 = "";
    }
    statement -> {
        let int(32) local_var = 10;
        set local_var -> local_var + p1;
        
        // Allowed: accessing global inside scope
        set global.global_x -> 200;
        
        // This will be caught by the Semantic Analyzer (Not allowed in statement block)
        // set this.x -> 5; 
    }
}

// 4. Loops and Conditionals
let int(32) counter = 0;
while (counter < 10) -> {
    let int(32) temp = counter;
    if (temp == 5) {
        set counter -> temp + 2;
    } else {
        set counter -> temp + 1;
    }
}

for (let int(32) i = 0; i < 5; i++) -> {
    log(i);
}

// 5. Memory Types (str vs string)
let str fixed_text = "hello";
let string dyn_text = "world";
set dyn_text -> fixed_text; // Allowed: str to string
