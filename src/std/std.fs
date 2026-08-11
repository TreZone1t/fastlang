export scope None -> {
    enable [custom_keyword, public];
    type -> custom;
    keyword -> "none";
    public -> {
        fn is_none() -> bool {
            return true;
        }
        fn is_some() -> bool {
            return false;
        }
    }
}

export scope Some -> {
    enable [private, public, constructor, custom_keyword, custom_generic];
    type -> custom;
    keyword -> "some";
    generic -> {
        type(size) T;
    }
    private -> {
        T value;
    }
    public -> {
        fn set_value(value: T) -> void {
            this.value = value;
        }
        fn get_value() -> T {
            return this.value;
        }
        fn is_some() -> bool {
            return true;
        }
        fn is_none() -> bool {
            return false;
        }
    }
    _(T value) -> {
        this.value -> value;
    }
}

export scope Option -> {
    enable [private, public, constructor, custom_keyword, custom_generic];
    type -> custom;
    keyword -> "option";
    generic -> {
        type(size) T;
    }
    private -> {
        T value;
        bool is_some;
    }
    public -> {
        fn set_value(value: T) -> void {
            this.value = value;
            this.is_some = true;
        }
        fn get_value() -> T {
            return this.value;
        }
        fn set_is_some(is_some: bool) -> void {
            this.is_some = is_some;
        }
        fn get_is_some() -> bool {
            return this.is_some;
        }
    }
    _() -> {
        this.is_some = false;
    }
}

//array<T>(length) name -> [ele1, ele2, ..., eleN];
export scope Array -> {
    type -> array;
    keyword -> "array";
    generic -> {
        type(size) T;
    }
    _(arr : name) -> {
        this.data -> arr; 
    }
    private -> {
        T data;
        int(32) length = len;
    }
    handle -> {
        fn index_access(index : int(32)) -> T {
            return this.data[index];
        }
    }
}
//str s -> "hello"; 
export scope Str -> { 
    type -> str;
    keyword -> "str";
    param -> {
        int(32) len;
    }
    private -> {
        name data;
        int(32) length = len;
    }
    _(arr : name) -> {
         this.data -> arr; 
    }
    handle -> {
        fn add(b : str) -> str {
            return this.data + b.data;
        }
        //data is has no use for now
        fn data() -> array<char> {
            return this.data;
        }
        fn length() -> int(32) {
            return this.length;
        }
    }
}
    