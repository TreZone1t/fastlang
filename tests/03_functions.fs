// Valid: Function definition and return
fn add_nums(a: int(32), b: int(32)) -> int(32) { 
    return a + b;
}
// Valid: Function definition and return using scope
scope ScopeAdd -> {
    type -> Fn;
    param -> {
        a: int(32);
        b: int(32);
    }
    statement -> {
        return a + b;
    }
    return -> int(32);
}
fn main() -> int(32) {
   Log(add_nums(1 , 2 ));
   Log(ScopeAdd(1 , 2 ));
    return 0;
}
