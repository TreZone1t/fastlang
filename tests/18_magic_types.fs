struct Node -> {
    public -> {
        int(32) value;
    }
       constructor -> {
        init(value : int(32)) ->{
         this.value = value;
        }
     }
 }
 fn test_magic(y: int(32)) -> void {
     // Test 1: modify with let
    name x = modify y;
    log(&x);
    // Test 2: copy with let
    int(32) arr2[3] = [1,2,3];
    // Test 3: Magic cast 

    int(32) temp = 10;
    Node n = new Node(temp);
}
fn main() -> int(32) {
    int(32) y = 10;
    test_magic(y);
    return 0;
};