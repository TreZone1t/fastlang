// Test 35: do-while loops with arrow syntax, standard block, and break/continue

fn test_do_while_arrow() -> int(32) {
    int(32) count = 0;
    int(32) sum = 0;
    do -> {
        count += 1;
        sum += count;
    } while (count < 5);
    return sum; // 1+2+3+4+5 = 15
}

fn test_do_while_standard() -> int(32) {
    int(32) val = 10;
    int(32) iterations = 0;
    do {
        iterations += 1;
        val -= 2;
        if (val <= 4) {
            break;
        }
    } while (val > 0);
    return iterations; // 10 -> 8 (1), 8 -> 6 (2), 6 -> 4 (3, breaks) => 3
}

fn test_do_while_single_stmt() -> int(32) {
    int(32) x = 3;
    do x -= 1; while (x > 0);
    return x; // 0
}

fn main() -> int(32) {
    int(32) r1 = test_do_while_arrow();
    int(32) r2 = test_do_while_standard();
    int(32) r3 = test_do_while_single_stmt();

    log("DoWhile Arrow Sum: ", r1);
    log("DoWhile Break Iterations: ", r2);
    log("DoWhile Single Stmt Final: ", r3);
    return 0;
}
