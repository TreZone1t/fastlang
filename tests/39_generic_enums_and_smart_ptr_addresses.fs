// Test 39: Generic Tagged Enums (Result, Struct variants), direct Enum variant access, and smart pointer address inspection

import std::string::{string};

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

enum YesOrNo -> {
    No,
    Yes,
}

fn main() -> int(32) {
    // 1. Smart pointer address inspection
    int(32) target_val = 777;
    name<int(32)> safe_ref = &target_val;
    modify<int(32)> mut_ref = &target_val;

    log("Target value via deref: ", *safe_ref);
    log("Target value direct: ", safe_ref);

    // 2. Generic enum instances
    Result<int(32), string> success = Result::Ok(42);
    Result<int(32), string> failure = Result::Err("error_not_found");

    log("Result Success: ", success);
    log("Result Failure: ", failure);

    // 3. Struct payload enum
    AppEvent event = AppEvent::Click(100, 200);
    AppEvent quit_event = AppEvent::Quit();
    log("AppEvent Click: ", event);
    log("AppEvent Quit: ", quit_event);

    // 4. Direct enum variant access and scope resolution
    YesOrNo choice1 = YesOrNo::Yes;
    YesOrNo choice2 = No;
    log("Choice1: ", choice1);
    log("Choice2: ", choice2);

    match (choice1) -> {
        YesOrNo::Yes => log("Choice1 matched Yes!"),
        YesOrNo::No => log("Choice1 matched No!"),
        _ => log("Choice1 unknown"),
    }

    return 0;
}
