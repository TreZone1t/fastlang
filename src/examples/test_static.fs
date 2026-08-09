struct Counter -> {
    static int count -> 0;
}
Counter.count -> Counter.count + 5;
log(Counter.count);
