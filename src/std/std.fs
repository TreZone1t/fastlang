export scope None -> {
    type -> custom;
    enable [ public];
    public -> {
        fn is_none() -> bool {
            return true;
        }
        fn is_some() -> bool {
            return false;
        }
    }
};

export scope Some -> {
    type -> custom;
    enable [private, public, constructor, custom_generic];
    generic -> {T;};
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
    constructor -> {
    init(value: T) -> {
        this.value = value;
    }
    }
}

export scope Option -> {
    type -> custom;
    enable [private, public, constructor,  custom_generic];
    generic -> {T;};
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
    constructor -> {
    init() -> {
        this.is_some = false;
    }
    }
}
//todo : make array and str as a primitive type
//array<T> name -> [ele1, ele2, ..., eleN];
/*
export scope Array -> {
    type -> ARRAY;
    generic -> {T;};
    private -> {
        Array<T> temp;
        int(32) len_temp;
    }
    init(arr:Array<T>) -> {
       this.temp -> arr;
    }
    handle -> {
        fn index_access(index : int(32)) -> T {
            return this.data[index];
        };
    }
    length -> 2;
    data -> this.temp;
}
//str s -> "hello"; 

export scope Str -> {
    type -> STR;
    param -> {
        int(32) len;
    }
    init(arr : name) -> {
         this.data -> arr; 
    }
    handle -> {
        fn add(b : Str) -> Str {
            return this.data + b.data;
        }
        //data is has no use for now
        fn data() -> Array<char> {
            return this.data;
        }
        fn length() -> int(32) {
            return this.length;
        }
    }
}
*/