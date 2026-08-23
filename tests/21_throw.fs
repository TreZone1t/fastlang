// 21_throw.fs
// 1. using handle to throw an error
custom IHaveThrow -> {
    handle -> {
        fn has_error(e : error) -> void {
            log("caught error : ", e);
        }
        fn call() -> void {
            throw new error("this is an error");
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