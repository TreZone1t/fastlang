export fn sum(a: int(32), b: int(32)) -> int(32) {
    return a + b;
}

fn private_helper() {
    log("this should not be visible to main.fs");
}
