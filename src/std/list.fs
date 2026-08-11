// Standard Library: List utilities
// using the scope system to define behavior
use std;

scope Node -> {
    enable [private, public, custom_keyword, custom_generic];
    type -> custom;
    keyword -> "node";
    generic -> {
        type(size) T;
    }
    private -> {
        T value;
        name next = null;  
    }
    public -> {
      fn set_next(node: name) -> void {
        this.next -> node;
      }
      fn get_next() -> name {
        return this.next;
      }
      fn set_value(value: T) -> void {
        this.value -> value;
      }
      fn get_value() -> T {
        return this.value;
      }
    }
    _(T value) -> {
         this.value -> value;
         this.next -> null;
    }
}

export scope LinkedList -> {
    enable [private, public, custom_index_access, handle, custom_generic, custom_keyword, length];
    type -> custom;
    keyword -> "list";
    generic -> {
        type(size) T;
    }
    private -> {
        name head = null;
        int(32) length = 0;
    }
    public -> {
      fn set_head(node: name) -> void {
        this.head -> node;
        this.length -> this.length + 1;
      }
      fn get_head() -> name {
        return this.head;
      }
      fn push(item: T) -> void {
        if (this.head == null) {
          this.head -> new node(item);
          this.length -> this.length + 1;
        } else {
          node<T> new_node = new node(item);
          new_node.set_next(this.head);
          this.head = new_node;
          this.length -> this.length + 1;
        }
      }
      fn pop() -> option<T> {
        let name temp = this.head;
        if (temp != null) {
          this.head -> temp.get_next();
          let T val = temp.get_value();
          del temp;
          this.length -> this.length - 1;
          
          option<T> result = new option();
          result.set_value(val);
          return result;
        } else {
          option<T> result = new option();
          return result;
        }
      }
      fn extend_from_array(arr: array<T>) -> void {
        for (let int(32) i = 0; i < arr.size; i++) -> {
          this.push(arr[i]);
        }
      }
    }
    handle -> {
      fn index_access(index: int(32)) -> option<T> {
        if (index < 0) {
            option<T> res = new option();
            return res;
        }
        let name temp = this.head;
        for (let int(32) i = 0; i < index; i++) -> {
          if (temp != null) {
              temp = temp.get_next();
          }
        }
        if (temp != null) {
            option<T> res = new option();
            res.set_value(temp.get_value());
            return res;
        } else {
            option<T> res = new option();
            return res;
        }
      }
      fn length() -> int(32) {
        return this.length;
      }
    }
    _(array<T> arr) -> {
        this.extend_from_array(arr);
    }
}
