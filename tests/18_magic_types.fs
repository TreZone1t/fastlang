scope main -> {
    type -> fn;
    statement -> {
        array(int(32), 3) arr1 = [1,2,3];
        int(32) y = 10;
        fn test_magic() -> void {
            // Test 1: modify with let
            let name x = modify y;
            log(&x);
            // Test 2: copy with let
            let array(int(32), 3) arr2 = copy(arr1);
            
            // Test 3: Magic cast 
            struct Node {
                public -> {
                   int(32) value;
                }
                _(int(32) value) ->{
                    this.value = value;
                }
            }
            let int(32) temp = 10;
            Node n = new Node(temp);
        }
        test_magic();
    }
    
}
