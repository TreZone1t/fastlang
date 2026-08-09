struct User -> {
    static int num = 0 ;
    public -> {
        int age;
        str name;
    }
    private -> {
    const str id;
     update() -> {
        num -> num + 1;
    }
    }
    _(name, age) : (str, int) -> {
        this.id -> num;
        this.name -> name;
        this.age -> age;
        this.update();
    }
}
fn main() -> int {
    let static num = User;
    let object user1 = User("hakim" , 19);
    let blueprint strt_fr_obj = {
        name : "hakim",
        age : 19,
        private.id : 1,
    }; //we can extract a blueprint scope for any object but it will not contain a constructor 
    scope sum -> { // we can type fn in this way 
        type -> Fn;
        param ->{
            int a;
            int b;
        }
        return -> (a + b);
    }
    log(sum(15,14));
    scope vec -> { //we can make our custom typed scope like a custom vec for a lib or something.
        type -> create("vec");
        public list<list> li;
        // i don't know how to make a vector
    }
    return 0;
}


//what the environment will be like that :
{
    name ->"filename";
    type -> global;
    public -> [{
          name ->"main";
          type -> Fn;
          ...
    }];
    private ->[{
         name ->"User";
          type -> Struct;
          static -> [{
            name -> "num";
            type -> Var ;
            dataType -> int;
            data -> 0;
          }]
          public -> [{
            name -> "age";
            type -> Var ;
            dataType -> int;
            size -> 16;
          } , {
            name -> "name";
            type -> Var ;
            dataType -> str;
            size -> flex ; // i don't no what will be here but it will indicate it a flexible size
          }]
          private -> [{
            name -> "id";
            type -> Const ;
            dataType -> str;
             size -> flex ;
          },{
          name ->"update";
          type -> Mthd;
           statement ->{
            User.num -> num + 1;;
           }
          }]
          ...  // i tried to show all the code but it will be hard , and i think this is enough.
    }];
}