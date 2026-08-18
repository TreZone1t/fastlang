
//1. using handle to throw an error
custom IHaveThrow -> {
    enable [handle , statement , call, error];
    statement -> {
        throw new error("this is an error");
    }
    handle -> {
        fn has_error(e : error) -> void {
            log("caught error : ", e);
        }
    }
};
//2. using throw without handle in try catch
fn IHaveThrow2() -> void {
    try -> {
        throw new error("this is an error");
    } catch (e) -> {
        log("caught error : ", e);
    }
}
fn main() -> int(32) {
    IHaveThrow ();
    IHaveThrow2 ();
    return 0;
}