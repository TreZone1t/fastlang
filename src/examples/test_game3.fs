struct Game -> {
    str name;
    int score;
    private int players -> 0;
    public update() -> {
        this.players -> this.players + 1;
    }
    _(name, score) : (str, int) -> {
        this.name -> name;
        this.score -> score;
        this.update();
    }
    getPlayers() -> {
        return this.players;
    }
}
let object g -> new Game("Naruto", 1000);
log(g.getPlayers());
