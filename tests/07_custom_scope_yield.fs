// Valid: complete custom scope with label , goto , yield , call , data
scope recursion_chaos_yield -> {
    type -> custom;
    enable [label,goto , yield  , call , data];
    add label -> @start;
    add label -> @process;
    data -> 0;
    @start -> {
      goto -> @process;
    }
    @process -> {
      this.data -> this.data + 2;
      yield;  // 2 , 4 , 6
      goto -> @process;
    }
    handle -> {
        fn call () -> int(32) {
            goto @start;
        }
        fn yield () -> int(32) {
           return this.data;
        }
    }
}

fn main() -> int(32) {
  log("Testing recursion_chaos_yield (yield state machine):");
  log(recursion_chaos_yield()); // Should print 2
  log(recursion_chaos_yield()); // Should print 4
  log(recursion_chaos_yield()); // Should print 6
  return 0;
}
