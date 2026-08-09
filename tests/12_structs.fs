// Valid: Basic struct
struct Point -> {
    public -> {
        int(32) x;
        int(32) y;
    }
    _(int(32) x, int(32) y) -> {
        this.x -> x;
        this.y -> y;
    }
}
