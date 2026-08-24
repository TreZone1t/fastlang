// Test 33: else if chains, inline if/else, and exhaustive return paths

fn evaluate_score(score: int(32)) -> int(32) {
    if (score >= 90) {
        return 1;
    } else if (score >= 75) {
        return 2;
    } else if (score >= 50) {
        return 3;
    } else {
        return 4;
    }
}

fn inline_check(cond_flag: bool) -> int(32) {
    if (cond_flag) return 100; else return 200;
}

fn main() -> int(32) {
    int(32) r1 = evaluate_score(85);
    int(32) r2 = inline_check(true);
    log("Score rank: ", r1);
    log("Inline rank: ", r2);
    return 0;
}
