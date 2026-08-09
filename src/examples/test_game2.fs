struct Game -> {
    str name;
    int score;
    bool isAlive;
    private int players -> 0;
    public update() -> {
        this.players -> this.players + 1;
    }
    _(name, score) : (str, int) -> {
        this.name -> name;
        this.score -> score;
        this.isAlive -> true;
        this.update();
    }
}
let object g -> new Game("Test", 100);
log(g.name);
