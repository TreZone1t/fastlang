/*
Error: Syntax Error: Expected a type, found 'blueprint'.
  --> line 1, column 1
   |
 1 | blueprint Point -> {int(32) x; int(32) y;};
   | ^
   |
Syntax Error: Expected a type, found 'blueprint'. at line 1, column 1
error: process didn't exit successfully: `target\debug\fast_lang.exe C:\Users\DELL\projects\fast_lang\tests\20_blueprint.fs` (exit code: 1)
*/
blueprint Point -> {int(32) x; int(32) y;};
impl Point -> {
    fn get_x() -> int(32) {
        return this.x;
    }
    fn get_y() -> int(32) {
        return this.y;
    }
};