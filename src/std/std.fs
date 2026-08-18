export custom None -> {
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

export custom Some<T> -> {

    enable [private, public, constructor, custom_generic];
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

export custom Option<T> -> {
    enable [private, public, constructor,  custom_generic];
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
