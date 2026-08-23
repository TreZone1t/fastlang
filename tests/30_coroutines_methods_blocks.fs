import std::coroutine::{go};

block counter -> {
    int(32) count = 0;
    for (int(32) i = 0; i < 5; i = i + 1) {
        count = count + 1;
        log("Counter step: ", count);
    }
}

fn add_nums(x: int(32), y: int(32)) -> int(32) {
    return x + y;
}

fn main() -> int(32) {
    // 1. Test method<void> and method inferred
    method<void> hello = () -> {
        log("hello from method");
    };
    hello();

    method goodbye = () -> log("goodbye from inferred method");
    goodbye();

    method add = (a: int(32), b: int(32)) -> {
        return a + b;
    };
    int(32) res = add(10, 20);
    log("Method add result: ", res);

    // 2. Test name<Fn<...>> function passing and calling
    name<Fn<(int(32), int(32)), int(32)>> fnRef = add_nums;
    int(32) fn_res = fnRef(15, 25);
    log("Fn ref result: ", fn_res);

    // 3. Test block scope and inspectable state
    counter();
    log("Final counter value: ", counter.count);

    // 4. Test coroutine / go
    go runner = new go(2);
    runner(counter);
    go(counter);

    return 0;
}
