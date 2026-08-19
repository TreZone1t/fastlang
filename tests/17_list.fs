// Test linked list standard library
import std::{ list};

fn main() -> void {
    list<int(32)> li -> [1,2,3,4,5,6];
    li.push(7);
    int(32) len = li.size();
    log("length : ", len); // length : 6
    li.push(8);
    log("length : ", len);  // length : 7
    for (int(32) i = 0; i < len; i = i + 1) -> {
        int(32) val = li.pop();
        log("popped value : ", val);
    }
}
