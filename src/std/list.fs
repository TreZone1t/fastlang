// Standard Library: List utilities
// using the scope system to define behavior

export class node<T> -> {
    private -> {
        T value;
        name<node<T>> next = null;
    }
    public -> {
        fn get_value() -> T {
            return this.value;
        }
        fn set_value(v: T) -> void {
            this.value = v;
        }
        fn get_next() -> name<node<T>> {
            return this.next;
        }
        fn set_next(n: name<node<T>>) -> void {
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

export class list<T> -> {
    private -> {
        name<node<T>> head = null;
        int(32) length = 0;
    }
    public -> {
      fn set_head(node_ptr: name<node<T>>) -> void {
        this.head = node_ptr;
        this.length = this.length + 1;
      }
      fn get_head() -> name<node<T>> {
        return this.head;
      }
      fn push(item: T) -> void {
        if (this.head == null) {
          this.head = new node<T>(item);
          this.length = this.length + 1;
        } else {
          modify<node<T>> new_node = new node<T>(item);
          new_node.set_next(this.head);
          this.head = new_node;
          this.length = this.length + 1;
        }
      }
      fn pop() -> T {
        modify<node<T>> temp = this.head;
        if (temp != null) {
          this.head = temp.get_next();
          T val = temp.get_value();
          del temp;
          this.length = this.length - 1;
          return val;
        }
        return 0;
      }
      fn extend_from_array(arr: T[]) -> void {
        for (int(32) i = 0; i < arr.length; i = i + 1) -> {
          this.push(arr[i]);
        }
      }
      fn size() -> int(32) {
        return this.length;
      }
    }
    handle -> {
      fn arrow(arr: T[]) -> void {
        this.extend_from_array(arr);
      }
      fn arrow_assign(arr: T[]) -> void {
        this.extend_from_array(arr);
      }
    }

    constructor -> {
      init(arr: T[]) -> {
        this.extend_from_array(arr);
      }
    }
}
