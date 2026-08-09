// 02_scopes.fs
// Test custom scopes, flags, settings, and constructors

scope DataBuffer -> {
    type -> custom;
    
    // Settings
    enable [index_access, length];
    
    // Control flags
    enable flag[is_break];
    
    // Dynamic fields
    add int(32) size;
    add int(32) capacity;
    
    // Constructor
    _(int(32) initial_capacity) -> {
        set this.capacity = initial_capacity;
        set this.size = 0;
    }
    
    public -> {
        fn get_size() -> int(32) {
            return this.size;
        }
    }
}
