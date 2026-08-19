custom NoIndex -> {
    enable [ handle];
handle -> {
    fn index_access(index : int(32)) -> int(32) {
        return 0;
    }
}
}
fn main() -> int(32) {
    NoIndex ni;  
    int(32) x = ni[0]; // Error: NoIndex does not support index_access
    return 0;
}
