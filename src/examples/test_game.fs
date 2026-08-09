struct Game -> {
    str name;
    int score;
    bool isAlive;
    private int players -> 0;
    public update() -> {
        this.players -> this.players + 1;
    }
}
let object g -> new Game();
