// 02_scopes.fs
// Test custom scopes, flags, settings, and constructors

custom DataBuffer -> {
    enable [index_access, length , private , public , constructor , operators , data];
    private -> {
    int(32) size;
    int(32) capacity;
    }
    // Constructor
    constructor -> {
    init(size : int(32)) -> {
    this.size -> size;
    this.capacity -> size;
       }
    }
    
    public -> {
        fn get_size() -> int(32) {
            return this.size;
        }
    }
    handle -> {
        fn index_access(index : int(32)) -> int(32) {
            if (index < this.size) {
                return this.data[index];
            } else {
                return 0;
            }
        }
        fn length() -> int(32) {
            return this.size;
        }
        fn add(value : int(32)) -> int(32) {
            if (this.size < this.capacity) {
                this.data[this.size] -> value;
                this.size -> this.size + 1;
                return value;
            } else {
                return 0;
            }
        }
}
}
