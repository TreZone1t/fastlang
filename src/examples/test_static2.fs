struct Counter -> {
    static int count -> 0;
    _(val) -> {
        Counter.count -> Counter.count + 1;
    }
}
let object a -> new Counter(10);
let object b -> new Counter(20);
log(Counter.count);
