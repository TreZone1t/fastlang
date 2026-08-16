// Valid: Basic struct
struct Point -> {
    public -> {
        int(32) x;
        int(32) y;
    }
    constructor -> {
    init(x : int(32), y : int(32)) -> {
        this.x -> x;
        this.y -> y;
    }
    }
}
