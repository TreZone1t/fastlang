/*

Error: Syntax Error: Unexpected token 'error' in expression
  --> line 1, column 1
   |
 1 | //1. using handle to throw an error
   | ^
   |
Syntax Error: Unexpected token 'error' in expression
*/
//1. using handle to throw an error
scope IHaveThrow -> {
    type -> custom;
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