struct Counter -> {
    static int count -> 0;
    private int id -> 0;
    public int value -> 0;
    _(val : int(16)) -> {
        this.value -> val;
        Counter.count -> Counter.count + 1;
        this.id -> Counter.count;
    }
}

let object a -> new Counter(10);
let object b -> new Counter(20);
log(Counter.count);
log(a.value, a.id);
log(b.value, b.id);

let public pub_a -> a;
log(pub_a.value);
