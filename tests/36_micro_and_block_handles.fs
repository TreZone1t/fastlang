// Test 36: micro declarations, execution, and clean keyword handles

micro Ilooped = (i: modify<int(32)>) -> {
    i -= 1;
};

micro double_it = (val: int(32)) -> int(32) {
    return val * 2;
};

custom CleanScope -> {
    data -> 100;
    @start -> {
        this.data -> this.data + 50;
        leave;
    }
    handle -> {
        fn call() -> int(32) {
            goto @start;
        }
        fn leave() -> int(32) {
            return this.data;
        }
        fn drop() -> void {
            log("CleanScope dropped cleanly");
        }
    }
}

block<int(32)> compute_sum -> {
    int(32) total = 0;
    for (int(32) k = 1; k <= 5; k = k + 1) {
        total = total + k;
    }
    return total;
}

fn main() -> int(32) {
    int(32) count = 5;
    Ilooped(&count);
    Ilooped(&count);
    log("Count after micro loops: ", count);

    int(32) doubled = double_it(21);
    log("Doubled value from micro: ", doubled);

    int(32) scope_res = CleanScope();
    log("CleanScope result: ", scope_res);

    int(32) block_sum = compute_sum();
    log("Block sum result: ", block_sum);
    log("Block internal total: ", compute_sum.total);

    return 0;
}
