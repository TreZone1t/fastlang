// Test linked list standard library
use std::list;

fn main() -> void {
    let list(int(32)) li = [1,2,3,4,5,6];
    li.push(7);
    let length len = li;
    for (let int(32) i = 0; i < len; i++) -> {
        let Option<int(32)> val = li.pop();
        log("popped value : ", val);
    }
}
