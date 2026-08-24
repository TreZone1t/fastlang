// 21_throw.fs
// 1. using handle to throw an error
custom IHaveThrow -> {
    @start -> {
        throw new error("this is an error using handle");
    }
    handle -> {
        fn call() -> void {
            goto @start;
        }
        fn error(e : error) -> void {
            log("caught error : ", e);
        }
    }
}

// 2. using throw without handle in try catch
fn IHaveThrow2() -> void {
    try -> {
        throw new error("this is an error");
    } catch (e) -> {
        log("caught error : ", e);
    }
}

fn main() -> int(32) {
    IHaveThrow();
    IHaveThrow2();
    return 0;
}