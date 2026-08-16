scope NoIndex -> {
    type -> custom;
    enable [index_access];
    disable [index_access];
}
fn main() -> int(32) {
    NoIndex ni;  
    int(32) x = ni[0]; // Error: NoIndex does not support index_access
    return 0;
}
