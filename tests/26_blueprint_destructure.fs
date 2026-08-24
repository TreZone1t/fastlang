blueprint Point -> {
    int(32) x;
    int(32) y;
};

fn IreturnPoint() -> Point {
    return Point { x = 10, y = 20 };
}

fn main() -> int(32) {
    {int(32) x; int(32) y;} = IreturnPoint();
    log(x); // EXPECT: 10
    log(y); // EXPECT: 20

    const {int(32) a; int(32) b;} = IreturnPoint();
    log(a); // EXPECT: 10
    log(b); // EXPECT: 20

    {c, d} = IreturnPoint();
    log(c); // EXPECT: 10
    log(d); // EXPECT: 20

    const {e, f} = IreturnPoint();
    log(e); // EXPECT: 10
    log(f); // EXPECT: 20

    return 0;
}
