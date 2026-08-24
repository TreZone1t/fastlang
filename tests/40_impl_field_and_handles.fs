// Test 40: impl <field> for <target> and comprehensive impl with handles

blueprint Point -> {
    int(32) x;
    int(32) y;
};

impl Point -> {
    fn get_x() -> int(32) {
        return this.x;
    }
    fn get_y() -> int(32) {
        return this.y;
    }
    handle -> {
        fn add(other: Point) -> Point {
            Point res;
            res.x = this.x + other.x;
            res.y = this.y + other.y;
            return res;
        }
    }
};

enum Status -> {
    Pending,
    Completed,
    Failed
}

impl handle for Status -> {
    fn display() -> string {
        match (this) -> {
            Status::Completed => { return "Done"; }
            _ => { return "Other"; }
        }
    }
}

fn compute(x: int(32)) -> int(32) {
    return x * 10;
}

impl handle for compute -> {
    fn error() -> bool {
        return false;
    }
}

fn main() -> int(32) {
    Point p1;
    p1.x = 10;
    p1.y = 20;

    Point p2;
    p2.x = 5;
    p2.y = 15;

    Point p3 = p1 + p2;
    log("Point sum X: ", p3.get_x());
    log("Point sum Y: ", p3.get_y());

    Status st = Status::Completed;
    log("Status st: ", st);

    int(32) ans = compute(7);
    log("Compute result: ", ans);

    return 0;
}
