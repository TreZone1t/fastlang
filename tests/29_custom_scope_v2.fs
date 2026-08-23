// Test Custom Scope v2 Features:
// 1. Direct labels without 'enable' or 'add label'
// 2. Data state container
// 3. Private fields & constructor

custom ConfigurableStateMachine -> {
    data -> 0;
    private -> {
        int(32) step;
        int(32) max_val;
    }
    constructor -> {
        init(step: int(32), max_val: int(32)) -> {
            this.step = step;
            this.max_val = max_val;
        }
    }

    @start -> {
        this.data -> this.data + this.step;
        if (this.data >= this.max_val) {
            leave;
        }
        goto -> @start;
    }

    handle -> {
        fn call() -> int(32) {
            goto -> @start;
        }
        fn leave() -> int(32) {
            return this.data;
        }
    }
}

fn main() -> int(32) {
    ConfigurableStateMachine sm = new ConfigurableStateMachine(3, 10);
    int(32) result = sm.call();
    log("Final SM Data: ", result); // EXPECT: Final SM Data: 12

    return 0;
}
