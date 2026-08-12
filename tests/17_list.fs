// Test linked list standard library
use std::list;

fn main() -> void {
    list<int(32)> li -> [1,2,3,4,5,6];
    li.push(7);
    int(32) len = li;
    for (int(32) i = 0; i < len; i++) -> {
        Option<int(32)> val -> li.pop();
        log("popped value : ", val);
    }
}
