// Test 38: Smart pointers ergonomics (name, modify, copy) and target value/address inspection

fn increment_val(m: modify<int(32)>) -> void {
    m += 10;
}

fn main() -> int(32) {
    int(32) num = 50;
    
    name<int(32)> read_ptr = &num;
    log("Value via name: ", read_ptr);
    log("Value via deref: ", *read_ptr);

    modify<int(32)> mod_ptr = &num;
    increment_val(mod_ptr);
    log("Value after increment: ", num);

    mod_ptr += 5;
    log("Value after direct +=: ", mod_ptr);

    log("Target pointee address via &*: ", &*mod_ptr);
    log("Target pointee address via *(&*: ", *(*&mod_ptr));

    return 0;
}
