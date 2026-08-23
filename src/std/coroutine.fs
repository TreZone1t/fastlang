// FastLang Standard Library: coroutine
export class go -> {
    public -> {
        int(32) capacity = 10;
    }

    constructor -> {
        init(cap: int(32)) -> {
            this.capacity = cap;
        }
        init(task: method<void>) -> {
            task();
        }
    }

    handle -> {
        fn call(task: method<void>) -> void {
            task();
        }
        fn call(task: method<int(32)>) -> int(32) {
            return task();
        }
    }
}
