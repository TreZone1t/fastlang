// Valid: Basic struct
struct Point -> {
    public -> {
        int(32) x;
        int(32) y;
    }
    _(x : int(32), y : int(32)) -> {
        this.x -> x;
        this.y -> y;
    }
}
