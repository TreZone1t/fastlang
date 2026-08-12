scope NoIndex -> {
    type -> custom;
    enable [custom_keyword];
    keyword -> "NoIndex";
}
fn main() -> int(32) {
    let NoIndex ni;  // error:  you should enable keyword to NoIndex to use it as type and add a string in custom()
    let int(32) x = ni[0]; // Error: NoIndex does not support index_access
    return 0;
}
