enum YesOrNo -> {
    No,
    Yes,
}

fn test_number_match(num: int(32)) -> int(32) {
    match (num) -> {
        1 => {
            return 100;
        }
        2 => {
            return 200;
        }
        _ => {
            return -1;
        }
    }
}

fn main() -> int(32) {
    int(32) num_res = test_number_match(2);
    log("Matched number result: ", num_res);

    YesOrNo answer = YesOrNo.Yes;
    log("Enum value: ", answer);

    match (answer) -> {
        YesOrNo.Yes => {
            log("Outcome: Affirmative!");
        }
        YesOrNo.No => {
            log("Outcome: Negative!");
        }
        _ => {
            log("Outcome: Unknown!");
        }
    }
    return 0;
}
