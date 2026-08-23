// FastLang Standard Library: string
export class string -> {
    private -> {
        char[] buffer = [];
        int(32) length = 0;
    }

    public -> {
        fn get_buffer() -> char[] {
            return this.buffer;
        }
    }

    handle -> {
        fn arrow(arr: char[]) -> void {
            this.buffer = arr;
            this.length = arr.size();
        }
        fn arrow_assign(arr: char[]) -> void {
            this.buffer = arr;
            this.length = arr.size();
        }
        fn size() -> int(32) {
            return this.length;
        }
        fn index_access(index: int(32)) -> char {
            return this.buffer[index];
        }
    }

    constructor -> {
        init(arr: char[]) -> {
            this.buffer = arr;
            this.length = arr.size();
        }
    }
}
