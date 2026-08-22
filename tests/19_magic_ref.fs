fn main() -> void {
    int(32) number = 42;
    int(32) my_array[5] = [1,2,3,4,5];

    name safe_ptr = &number; 

    modify<int(32)> mut_ptr = &number;

    int(32)* my_heap_array = new int(32)[1,2,3,4,5];
    
    modify<int(32)> not_safe_ptr = &my_heap_array[0];
     del not_safe_ptr;  // now it will give error : cannot delete name
    not_safe_ptr = my_array;   //it doesn't work *error: assignment of read-only reference 'not_safe_ptr'*
    
    name<int(32)> bad_ptr = safe_ptr;   //Warning : nested and linked ptr you must to be manged correctly
    log(*bad_ptr);
    log(*mut_ptr);
}