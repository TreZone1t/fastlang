let int(32) x = 10;
let string y = "hello";

// Should fail: redeclaration
// let int(32) x = 20;

// Should fail: type mismatch
// let int(32) z = "world";

// Should fail: invalid magic type usage (length on int)
// let length l = x;

// Should fail: missing variable
let int(32) a = unknown_var;
