// Standard Library: List utilities
// using the scope system to define behavior


export custom node<T> -> {
    enable [oop, custom_generic];
    private -> {
        T value;
        name next = null;
    }
    public -> {
        fn get_value() -> T {
            return this.value;
        }
        fn set_value(v: T) -> void {
            this.value = v;
        }
        fn get_next() -> name {
            return this.next;
        }
        fn set_next(n: name) -> void {
            this.next = n;
        }
    }
    constructor -> {
        init(v: T) -> {
            this.value = v;
            this.next = null;
        }
    }
}

export custom list<T> -> {
    enable [oop, custom_index_access, handle, custom_generic, length, custom_constructor];
    private -> {
        name<node> head;
        int(32) length = 0;
    }
    public -> {
      fn set_head(node_ptr: name) -> void {
        this.head = node_ptr;
        this.length = this.length + 1;
      }
      fn get_head() -> name {
        return this.head;
      }
      fn get_node(index: int(32)) -> name {
        if (index < 0) {
          return void;
        }
        name temp = this.head;
        for (int(32) i = 0; i < index; i = i + 1) -> {
          if (temp != void) {
              temp = temp.get_next();
          }
        }
        res.set_value(temp.get_value());
        return res;
      }
      fn push(item: T) -> void {
        if (this.head == void) {
          this.head = new node(item);
          this.length = this.length + 1;
        } else {
          node<T> new_node = new node(item);
          new_node.set_next(this.head);
          this.head = new_node;
          this.length = this.length + 1;
        }
      }
      fn size() -> int(32) {
        return this.length;
      }
      fn pop() -> T {
        let name temp = this.head;
        if (temp !=  void) {
          this.head = temp.get_next();
          let T val = temp.get_value();
          del temp;
          this.length = this.length - 1;
          return val;
        }
        return 0;
      }
      fn extend_from_array(arr: T[]) -> void {
        for (let int(32) i = 0; i < arr.size(); i = i + 1) -> {
          this.push(arr[i]);
        }
      }
    }
    handle -> {
      fn arrow_assign(arr: T[]) -> void {
        this.extend_from_array(arr);
      }
      fn index_access(index: int(32)) -> name {
        if (index < 0) {
           T res = this.get_node(index);
            return res;
        }
         name temp ->  void;
           return temp;
        }

      fn size() -> int(32) {
        return this.length;
      }
    }
    constructor -> {
      init(arr: T[]) -> {
        this.extend_from_array(arr);
      }
    }

}
