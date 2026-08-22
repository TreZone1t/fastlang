custom NoIndex -> {
    enable [ handle];
handle -> {
}
}
fn main() -> int(32) {
    NoIndex ni;  
    int(32) x = ni[0]; // Error: NoIndex does not support index_access
    return 0;
}
