let int x = 15;
if (x > 10) -> {
    log("Big");
} else -> {
    log("Small");
}

struct Player -> {
    name : str;
    score : int;
    isAlive -> true;
    _(name, score) : (str, int) -> {
        this.name -> name;
        this.score -> score;
    }
}

let object p = new Player("Hakim", 99);
log(p.name, p.score, p.isAlive);
