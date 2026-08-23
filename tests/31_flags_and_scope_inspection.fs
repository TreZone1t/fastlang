// Test 31: Flags, Yield State Machine, and Scope Magic Type Inspection

fn add_numbers(a: int(32), b: int(32)) -> int(32) {
    return a + b;
}

block generator -> {
    int(32) step = 0;
    step = step + 1;
    yield;
    step = step + 1;
    yield;
}

fn main() -> int(32) {
    // 1. Test custom flag
    flag run_on_gpu = true;
    flag is_debug = false;
    log("Flag run_on_gpu: ", run_on_gpu);
    log("Flag is_debug: ", is_debug);

    // 2. Test scope<T> magic wrapper on function
    scope<Fn<(int(32), int(32)), int(32)>> s = add_numbers;
    int(32) result = s(15, 25);
    log("Function result: ", result);
    log("Scope has_return: ", s.has_return());
    log("Scope return_value: ", s.return_value);
    log("Scope is_done: ", s.is_done());

    // 3. Test block state machine with yield
    generator();
    log("Generator step 1: ", generator.step);
    log("Generator has_yielded: ", generator.has_yielded);

    generator();
    log("Generator step 2: ", generator.step);

    generator();
    log("Generator is_done: ", generator.is_done);

    return 0;
}
