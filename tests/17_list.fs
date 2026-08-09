// Test linked list standard library
use std::list;

fn main() -> void {
    let list(int(32)) li = [1,2,3,4,5,6];
    li.push(7);

    // Pop the nodes
    let Option<int(32)> val1 = li.pop();
    let Option<int(32)> val2 = li.pop();
    let Option<int(32)> val3 = li.pop();
    let Option<int(32)> val4 = li.pop();
    let Option<int(32)> val5 = li.pop();
    let Option<int(32)> val6 = li.pop();
    let Option<int(32)> val7 = li.pop();
    let Option<int(32)> val8 = li.pop();

}
