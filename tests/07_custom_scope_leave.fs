// Valid: complete custom scope with label , goto , leave , call , data
scope recursion_chaos -> {
    type -> custom;
    enable [label,goto , leave , call , data];
    add label -> @start;
    add label -> @process;
    data -> 0;
    @start -> {
      if (this.data <= 0) {
        this.data -> 1;
      }else{
        this.data -> this.data * 2;
      }
      goto -> @process;
    }
    @process -> {
      if ( this.data % 2 == 0) {
        this.data -> this.data + 1;
      }else{
        leave;
      }
      goto -> @start;
    }
    handle -> {
        fn call () -> int(32) {
            goto  -> @start;
        }
        fn leave () -> int(32) {
           return this.data;
        }
    }
}

fn main() -> int(32) {
  log("Testing recursion_chaos (leave):");
  log(recursion_chaos()); // Should print 1
  return 0;
}
