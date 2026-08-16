scope Main -> {
    type -> fn;
    statement -> {
        int(32) y = 10;
        fn test_magic() -> void {
            // Test 1: modify with let
            name x = modify y;
            log(&x);
            // Test 2: copy with let
            array<int(32)> arr2 -> [1,2,3];
            // Test 3: Magic cast 
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
            int(32) temp = 10;
            Node n -> new Node(temp);
    }
    return 0;
    };
    return -> int(32);
    };