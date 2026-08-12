// 02_scopes.fs
// Test custom scopes, flags, settings, and constructors

scope DataBuffer -> {
    type -> custom;
    
    // Settings

    enable [index_access, length];
    private -> {
    int(32) size;
    int(32) capacity;
    }
    // Constructor
    _(capacity : int(32)) -> {
        this.capacity -> capacity;
        this.size -> 0;
    }
    
    public -> {
        fn get_size() -> int(32) {
            return this.size;
        }
    }
}
