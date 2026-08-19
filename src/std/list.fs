// Standard Library: List utilities
// using the scope system to define behavior
use std;

export custom list<T> -> {
    enable [oop, custom_index_access, handle, custom_generic, length , custom_constructor];
    private -> {
        name head = null;
        int(32) length = 0;
    }
    public -> {
      fn set_head(node: name) -> void {
        this.head = node;
        this.length = this.length + 1;
      }
      fn get_head() -> name {
        return this.head;
      }
      fn push(item: T) -> void {
        if (this.head == null) {
          this.head = new node(item);
          this.length = this.length + 1;
        } else {
          node<T> new_node = new node(item);
          new_node.set_next(this.head);
          this.head = new_node;
          this.length = this.length + 1;
        }
      }
      fn pop() -> option<T> {
        let name temp = this.head;
        if (temp != null) {
          this.head = temp.get_next();
          let T val = temp.get_value();
          del temp;
          this.length = this.length - 1;
          
          option<T> result -> new Option();
          result.set_value(val);
          return result;
        } else {
          option<T> result = new Option();
          return result;
        }
      }
      fn extend_from_array(arr: T[]) -> void {
        for (let int(32) i = 0; i < arr.size; i++) -> {
          this.push(arr[i]);
        }
      }
    }
    handle -> {
      // custom constructor example now we can do this
      // LinkedList<int(32)> li -> [1,2,3];
      //before we had to do this
      // LinkedList<int(32)> li = new LinkedList([1,2,3]);
      fn arrow_assign(arr: T[]) -> void {
        this.extend_from_array(arr);
      }
      fn index_access(index: int(32)) -> Option<T> {
        if (index < 0) {
            Option<T> res -> new Option();
            return res;
        }
        name temp = this.head;
        for (int(32) i = 0; i < index; i++) -> {
          if (temp != null) {
              temp = temp.get_next();
          }
        }
        if (temp != null) {
            Option<T> res -> new Option();
            res.set_value(temp.get_value());
            return res;
        } else {
            Option<T> res -> new Option();
            return res;
        }
      }
      fn length() -> int(32) {
        return this.length;
      }
    }
    constructor -> {
    init(arr: T[]) -> {
        this.extend_from_array(arr);
    }
    }
}
