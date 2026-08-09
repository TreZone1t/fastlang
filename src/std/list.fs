// Standard Library: List utilities
// using the scope system to define behavior
scope Node -> {
    enable [private, public, init];
    type -> custom("node");
    param -> {
        type(size) T;
    }
    private -> {
        T value;
        name next = null;  
    }
    public -> {
      fn set_next(node: name) -> void{
        this.next = node;
      }
      fn get_next() -> name {
        return this.next;
      }
      fn set_value(value: T) -> void{
        this.value = value;
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
    enable [private , public,param , init,keyword];
    type -> custom("list");
    private -> {
        name head = null;
    }
    public -> {
      fn set_head(node: name) -> void{
        this.head = node;
      }
      fn get_head() -> name {
        return this.head;
      }
      fn push(item: T) -> void{
        if (this.head == null ){
          this.head = new Node(item);
        }else{
          let name new_node = new Node(item);
          // Hack: we prepend to make it simple
          new_node.set_next(this.head);
          this.head = new_node;
        }
      }

      fn pop() -> Option<T>{
        let name temp = this.head;
        if (temp != null) {
          this.head = temp.get_next();
          let T val = temp.get_value();
          del temp;
          return Some(val);
        }
        else {
            return None;
        }
      }
    }
    //extending this list with elments from an array
    fn extend_from_array (arr: array(T, size)) -> void {
      for (let int(32) i = 0;i < arr.size; i++) -> {
        this.push(arr[i]);
      }
    }
    //this way combine with keyword allow to be declared as list(int(32)) 
    param -> {
        type(size) T;
    }
    //and any elment came from the constructor when we allow keyword will be one arg came after ->
    _(array(T, size) arr) -> {
        this.extend_from_array(arr);
    }
}
