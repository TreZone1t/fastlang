// Test 41: Enum Payload & Blueprint Pattern Matching with Destructuring

import std::string::{string};

blueprint Point -> {
    int(32) x;
    int(32) y;
};

fn make_point(px: int(32), py: int(32)) -> Point {
    return Point { x = px, y = py };
}

enum Result<T, E> -> {
    Ok(T),
    Err(E),
}

enum AppEvent -> {
    Click {
        int(32) x;
        int(32) y;
    },
    Quit,
}

fn process_result(res: Result<int(32), string>) -> int(32) {
    int(32) output = 0;
    match (res) -> {
        Result::Ok(val) => {
            log("Match Ok with value: ", val);
            output = val;
        }
        Result::Err(err) => {
            log("Match Err: ", err);
            output = -1;
        }
        _ => {
            log("Default result match");
            output = 0;
        }
    }
    return output;
}

fn process_event(evt: AppEvent) -> int(32) {
    int(32) score = 0;
    match (evt) -> {
        AppEvent::Click(x, y) => {
            log("Match Click: x=", x, " y=", y);
            score = x + y;
        }
        AppEvent::Quit => {
            log("Match Quit event");
            score = 999;
        }
        _ => {
            log("Unknown event");
        }
    }
    return score;
}

fn main() -> int(32) {
    // 1. Inferred blueprint destructuring
    {px, py} = make_point(100, 200);
    log("Destructured Point: ", px, ", ", py);

    const {cx, cy} = make_point(300, 400);
    log("Const Destructured Point: ", cx, ", ", cy);

    // 2. Enum Tuple Payload pattern matching
    Result<int(32), string> res_ok = Result::Ok(777);
    int(32) r1 = process_result(res_ok);
    log("Process Result Ok output: ", r1);

    Result<int(32), string> res_err = Result::Err("network_timeout");
    int(32) r2 = process_result(res_err);
    log("Process Result Err output: ", r2);

    // 3. Enum Struct Payload pattern matching
    AppEvent click_evt = AppEvent::Click(15, 25);
    int(32) e1 = process_event(click_evt);
    log("Process Click Event output: ", e1);

    AppEvent quit_evt = AppEvent::Quit();
    int(32) e2 = process_event(quit_evt);
    log("Process Quit Event output: ", e2);

    return 0;
}
