// Standard Library: List utilities
// using the scope system to define behavior

export scope list_utils -> {
    enable [param , init]; // to enable the param and constructor
    type -> custom("list");
    //the string in custom will be use as keyword like this 
    // list item -> { ... } or even if we add more custom things 
    // like list[size](type) item -> [];  or 
    // like list[size] item -> (type)[]; or list<type> item -> [];
    // we no longer enable length, we explicitly add it as a field
    // wait, we can also enable flag[is_drop] if we want
    
    add int(32) length;
    add int(32) size;
    add type dataType;
    param -> {
        type T;
    }
    _(array(T) arr) -> {
        set this.dataType -> T;
        set this.size -> arr.length;
        set this.length -> arr.length;
        // this.extend(arr); //TODO: implement this
        
    }
    
    // but i thick it better to separate 
    // actually now we will call that way
    // list(int(4)) arr -> []; //* but i don't know how to make it accept [] not {}
    // so actually it is better to make a primative type like arrey that need a type and size and list unlike our list
    // let array(int(4)) arr = [1,2,3,4];
    // list(int(4)) list -> arr; or list(int(4)) list -> [1,2,3,4];
    public -> {
        fn push(item: int(32)) {
            // Native array push
            // this.length = this.length + 1;
        }
    }
}
