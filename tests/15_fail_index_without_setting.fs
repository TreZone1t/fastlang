// Invalid: index access without setting
scope NoIndex -> {
    type -> custom;
    // index_access not enabled
    add int(32) val;
}
fn main() -> int(32) {
    let NoIndex ni;
    let int(32) x = ni[0]; // Error: NoIndex does not support index_access
    return 0;
}
