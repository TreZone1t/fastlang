struct Point -> {
    int x;
    int y;
    _(px, py) -> {
        this.x -> px;
        this.y -> py;
    }
}

let object p -> new Point(10, 20);
log(p.x, p.y);
