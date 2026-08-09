class Dog -> {
    public int(32) age;
    public -> {
        _() -> {
            set this.age -> 5;
        }
    }
}

// Global scope array test
let list arr = [1, 2, 3];
log(arr[0]);
let int(32) b = -arr[1];
let bool c = !true;

let object obj1 = new Dog();
let object obj2 = copy obj1;
let object obj3 = modify obj1;
